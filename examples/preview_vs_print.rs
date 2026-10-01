//! Render special attractors as the live preview shows them and as an A3 print, so the two can
//! be compared by eye (shrink the print to the preview's size to compare overall brightness).
//!
//! cargo run --release --example preview_vs_print -- <out_dir> <palette index> [special line numbers...]
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use attractors::render::{blend_and_colour, write_png, Look, SAVE_STEPS_PER_PIXEL};
use attractors::*;

const PREVIEW: (usize, usize) = (636, 900);
const PREVIEW_STEPS: usize = 10_000_000;
const PRINT: (usize, usize) = (7016, 9933);

fn histogram(attractor: &CliffordAttractor, size: (usize, usize), steps: usize) -> Histogram {
    let mut hist = Histogram::new(size.0, size.1);
    let mut sampler = Sampler::default();
    while hist.steps < steps {
        let chunk = (steps - hist.steps).min(50_000_000);
        sampler.step(attractor, chunk, &mut hist, &View::default());
    }
    hist
}

fn write_preview(hist: &Histogram, look: &Look, path: &Path) {
    let mut prev = vec![0.0; hist.counts.len()];
    let mut buffer = vec![0u32; hist.counts.len()];
    blend_and_colour(hist, &mut prev, 1.0, look, &mut buffer);
    let file = BufWriter::new(File::create(path).unwrap());
    let mut encoder = png::Encoder::new(file, hist.width as u32, hist.height as u32);
    encoder.set_color(png::ColorType::Rgb);
    let rgb: Vec<u8> = buffer
        .iter()
        .flat_map(|c| [(c >> 16) as u8, (c >> 8) as u8, *c as u8])
        .collect();
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&rgb)
        .unwrap();
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = Path::new(args.get(1).map_or("cache/compare", String::as_str));
    std::fs::create_dir_all(out).unwrap();
    let look = Look {
        palette: args.get(2).map_or(0, |s| s.parse().unwrap()),
        ..Look::default()
    };
    let specials: Vec<[f64; 4]> = std::fs::read_to_string("cache/clifford/special.txt")
        .unwrap()
        .lines()
        .filter_map(parse_params)
        .collect();
    let lines: Vec<usize> = if args.len() > 3 {
        args[3..].iter().map(|s| s.parse().unwrap()).collect()
    } else {
        vec![0, specials.len() / 2]
    };

    for line in lines {
        let attractor = CliffordAttractor::new(specials[line]);
        let preview = histogram(&attractor, PREVIEW, PREVIEW_STEPS);
        write_preview(&preview, &look, &out.join(format!("{line}_preview.png")));

        let print_steps = (PRINT.0 as f64 * PRINT.1 as f64 * SAVE_STEPS_PER_PIXEL) as usize;
        let print = histogram(&attractor, PRINT, print_steps);
        let path = out.join(format!("{line}_print.png"));
        write_png(&print, &look, &path).unwrap();
        println!("{line}: wrote {} and the preview", path.display());
    }
}
