use attractors::render::{blend_and_colour, write_png};
use attractors::{Attractor, CliffordAttractor, Histogram, View};

const PREVIEW: (usize, usize) = (636, 900);
const A5: (usize, usize) = (3506, 4960);
const POWER: f64 = 0.15;

fn main() {
    divan::main();
}

fn warm_histogram(size: (usize, usize), steps: usize) -> (CliffordAttractor, Histogram) {
    let mut c = CliffordAttractor::new(CliffordAttractor::DEFAULT_PARAMS);
    let mut hist = Histogram::new(size.0, size.1);
    c.step(steps, &mut hist, &View::default());
    (c, hist)
}

/// The work added each frame until the preview has MAX_PREVIEW_STEPS points.
#[divan::bench]
fn step_500k(bencher: divan::Bencher) {
    let (mut c, mut hist) = warm_histogram(PREVIEW, 0);
    bencher.bench_local(|| c.step(500_000, &mut hist, &View::default()));
}

/// Normalising and colouring the preview, done every frame.
#[divan::bench]
fn colour_preview_frame(bencher: divan::Bencher) {
    let (_, hist) = warm_histogram(PREVIEW, 10_000_000);
    let mut prev = vec![0.0; PREVIEW.0 * PREVIEW.1];
    let mut buffer = vec![0u32; PREVIEW.0 * PREVIEW.1];
    bencher.bench_local(|| blend_and_colour(&hist, &mut prev, 0.9, POWER, &mut buffer));
}

/// What a parameter change costs before the first new frame can be drawn.
#[divan::bench]
fn parameter_change(bencher: divan::Bencher) {
    let (mut c, mut hist) = warm_histogram(PREVIEW, 10_000_000);
    let mut a = c.a;
    bencher.bench_local(|| {
        a += 1e-3;
        c.set_params([Some(a), None, None, None]);
        hist.clear();
        c.step(500_000, &mut hist, &View::default());
    });
}

#[divan::bench(sample_count = 5)]
fn write_png_a5(bencher: divan::Bencher) {
    let (_, hist) = warm_histogram(A5, A5.0 * A5.1);
    let path = std::env::temp_dir().join("attractors_bench.png");
    bencher.bench_local(|| write_png(&hist, POWER, &path).unwrap());
    let _ = std::fs::remove_file(path);
}
