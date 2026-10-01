//! Search for chaotic parameters of an attractor and render the best candidates.
//!
//! Parameters are multiples of 0.1 across `PARAM_RANGE`, as in Sprott's letter codes. A candidate
//! must be chaotic (positive Lyapunov exponent) and its orbit must cover a reasonable fraction of
//! its bounding box, which rules out thin curves and scattered dots.
//!
//! Usage: cargo run --release --example find_chaotic -- <out dir> [count]

use std::path::Path;

use attractors::render::write_png;
use attractors::{
    format_params, lyapunov_exponent, Attractor, Histogram, Sampler, SprottQuadraticAttractor, View,
};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

type Searched = SprottQuadraticAttractor;

/// Fraction of a 100x100 grid over the orbit's bounding box that the orbit visits
fn coverage<A: Attractor>(attractor: &A) -> f64 {
    let mut sampler = Sampler::with_walkers(1, A::START);
    let mut hist = Histogram::new(100, 100);
    sampler.step(attractor, 50_000, &mut hist, &View::default());
    hist.counts.iter().filter(|&&c| c > 0).count() as f64 / hist.counts.len() as f64
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out_dir = Path::new(args.get(1).expect("usage: find_chaotic <out dir> [count]"));
    let count: usize = args.get(2).map_or(8, |s| s.parse().unwrap());
    std::fs::create_dir_all(out_dir).unwrap();

    let (lo, hi) = Searched::PARAM_RANGE;
    let steps = ((hi - lo) / 0.1).round() as i32;
    let mut rng = StdRng::seed_from_u64(0);
    let mut found = 0;
    let mut tries = 0;
    while found < count {
        tries += 1;
        let params: Vec<f64> = (0..Searched::PARAM_NAMES.len())
            .map(|_| ((lo + 0.1 * rng.gen_range(0..=steps) as f64) * 10.0).round() / 10.0)
            .collect();
        let attractor = Searched::from_params(&params);
        let lyapunov = lyapunov_exponent(&attractor, 5_000);
        if lyapunov.is_nan() || lyapunov <= 0.05 {
            continue;
        }
        let covered = coverage(&attractor);
        if covered < 0.15 {
            continue;
        }
        let mut sampler = Sampler::new(Searched::START);
        let mut hist = Histogram::new(400, 400);
        sampler.step(&attractor, 4_000_000, &mut hist, &View::default());
        let path = out_dir.join(format!("{found:02}.png"));
        write_png(&hist, 0.15, &path).unwrap();
        println!(
            "{found:02} after {tries} tries: lyapunov={lyapunov:.3} coverage={covered:.2} {}",
            format_params(Searched::PARAM_NAMES, &params)
        );
        found += 1;
    }
}
