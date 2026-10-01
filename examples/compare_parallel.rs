//! Does splitting stepping across cores give the same picture as one long trajectory?
//!
//! For a spread of parameters from special.txt, builds three histograms: one trajectory from
//! (0, 0), one from a different start (the noise floor between two honest single-core runs), and
//! the multi-core `step`. Reports total variation distance (half the L1 distance between the
//! normalised histograms) at several step counts, and writes images plus an HTML page.
//!
//! Usage: cargo run --release --example compare_parallel -- <out dir> [num params] [steps]

use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

use attractors::render::write_png;
use attractors::{
    format_params, parse_params, Attractor, CliffordAttractor, Histogram, Sampler, View,
};

const SIZE: usize = 400;
const POWER: f64 = 0.15;
const CHECKPOINTS: usize = 4;

fn tv_distance(a: &Histogram, b: &Histogram) -> f64 {
    let sum_a: f64 = a.counts.iter().map(|&c| c as f64).sum();
    let sum_b: f64 = b.counts.iter().map(|&c| c as f64).sum();
    if sum_a == 0.0 || sum_b == 0.0 {
        return f64::NAN;
    }
    0.5 * a
        .counts
        .iter()
        .zip(&b.counts)
        .map(|(&x, &y)| (x as f64 / sum_a - y as f64 / sum_b).abs())
        .sum::<f64>()
}

struct Run {
    attractor: CliffordAttractor,
    sampler: Sampler,
    hist: Histogram,
}

impl Run {
    /// `None` uses the default multi-core sampler, otherwise one trajectory from `start`
    fn new(params: [f64; 4], start: Option<(f64, f64)>) -> Self {
        Run {
            attractor: CliffordAttractor::new(params),
            sampler: start.map_or_else(Sampler::default, |s| Sampler::new(1, s)),
            hist: Histogram::new(SIZE, SIZE),
        }
    }

    fn advance(&mut self, steps: usize) {
        self.sampler
            .step(&self.attractor, steps, &mut self.hist, &View::default());
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out_dir = Path::new(
        args.get(1)
            .expect("usage: compare_parallel <out dir> [n] [steps]"),
    );
    let n_params: usize = args.get(2).map_or(16, |s| s.parse().unwrap());
    let total_steps: usize = args.get(3).map_or(40_000_000, |s| s.parse().unwrap());
    fs::create_dir_all(out_dir).unwrap();

    let specials: Vec<[f64; 4]> = fs::read_to_string("cache/clifford/special.txt")
        .unwrap()
        .lines()
        .filter_map(parse_params)
        .collect();
    let mut chosen = vec![CliffordAttractor::DEFAULT_PARAMS];
    chosen.extend((0..n_params - 1).map(|i| specials[i * specials.len() / (n_params - 1)]));

    let checkpoints: Vec<usize> = (1..=CHECKPOINTS)
        .map(|i| total_steps >> (2 * (CHECKPOINTS - i)))
        .collect();
    let mut jsonl = BufWriter::new(File::create(out_dir.join("results.jsonl")).unwrap());
    let mut rows = String::new();

    for (idx, &params) in chosen.iter().enumerate() {
        let mut runs = [
            Run::new(params, Some((0.0, 0.0))),
            Run::new(params, Some((1.3, -0.7))),
            Run::new(params, None),
        ];
        let mut done = 0;
        let mut cells = String::new();
        let mut json_checkpoints = Vec::new();
        for &checkpoint in &checkpoints {
            for run in &mut runs {
                run.advance(checkpoint - done);
            }
            done = checkpoint;
            let noise = tv_distance(&runs[0].hist, &runs[1].hist);
            let parallel_a = tv_distance(&runs[0].hist, &runs[2].hist);
            let parallel_b = tv_distance(&runs[1].hist, &runs[2].hist);
            json_checkpoints.push(format!(
                "{{\"steps\":{checkpoint},\"single_vs_single\":{noise:.5},\"single_a_vs_multi\":{parallel_a:.5},\"single_b_vs_multi\":{parallel_b:.5}}}"
            ));
            cells.push_str(&format!(
                "<td>{:.1}M</td><td>{noise:.4}</td><td>{parallel_a:.4}</td><td>{parallel_b:.4}</td></tr><tr>",
                checkpoint as f64 / 1e6
            ));
        }
        let names = ["single_a", "single_b", "multi"];
        for (run, name) in runs.iter().zip(names) {
            write_png(
                &run.hist,
                POWER,
                &out_dir.join(format!("{idx:02}_{name}.png")),
            )
            .unwrap();
        }
        writeln!(
            jsonl,
            "{{\"idx\":{idx},\"params\":\"{}\",\"checkpoints\":[{}]}}",
            format_params(params),
            json_checkpoints.join(",")
        )
        .unwrap();
        jsonl.flush().unwrap();

        let images: String = names
            .iter()
            .map(|n| format!("<img src=\"{idx:02}_{n}.png\" width=\"260\" title=\"{n}\">"))
            .collect();
        let cells = cells.trim_end_matches("<tr>");
        rows.push_str(&format!(
            "<tr><td rowspan=\"{CHECKPOINTS}\"><code>{}</code><br>{images}</td>{cells}\n",
            format_params(params)
        ));
    }

    let html = format!(
        r#"<!doctype html><meta charset="utf-8"><title>Single vs multi-core</title>
<style>body{{font-family:system-ui;background:#111;color:#ddd;margin:16px}}
table{{border-collapse:collapse}}td,th{{border:1px solid #333;padding:4px 8px;vertical-align:top}}
img{{margin:2px;image-rendering:pixelated}}code{{font-size:12px}}</style>
<h1>One trajectory vs multi-core trajectories</h1>
<p>Images left to right: one trajectory from (0, 0), one trajectory from (1.3, -0.7), multi-core.
Numbers are total variation distance between normalised histograms ({SIZE}x{SIZE}): 0 means
identical distributions, 1 means no overlap. "Single vs single" is the noise floor between two
honest one-core runs; multi-core matches if its columns are no larger.</p>
<table><tr><th>Parameters</th><th>Steps</th><th>Single vs single</th><th>Single (0,0) vs multi</th><th>Single (1.3,-0.7) vs multi</th></tr>
{rows}</table>"#
    );
    fs::write(out_dir.join("index.html"), html).unwrap();
    println!("{}", out_dir.join("index.html").display());
}
