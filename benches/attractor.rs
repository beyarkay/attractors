use attractors::render::{blend_and_colour, write_png};
use attractors::{Attractor, CliffordAttractor, Histogram, Sampler, View};

const PREVIEW: (usize, usize) = (636, 900);
const A5: (usize, usize) = (3506, 4960);
const POWER: f64 = 0.15;

fn main() {
    divan::main();
}

fn warm_histogram(size: (usize, usize), steps: usize) -> (CliffordAttractor, Sampler, Histogram) {
    let c = CliffordAttractor::from_params(CliffordAttractor::DEFAULT_PARAMS);
    let mut sampler = Sampler::new(CliffordAttractor::START);
    let mut hist = Histogram::new(size.0, size.1);
    sampler.step(&c, steps, &mut hist, &View::default());
    (c, sampler, hist)
}

/// The work added each frame until the preview has MAX_PREVIEW_STEPS points.
#[divan::bench]
fn step_500k(bencher: divan::Bencher) {
    let (c, mut sampler, mut hist) = warm_histogram(PREVIEW, 0);
    bencher.bench_local(|| sampler.step(&c, 500_000, &mut hist, &View::default()));
}

/// Normalising and colouring the preview, done every frame.
#[divan::bench]
fn colour_preview_frame(bencher: divan::Bencher) {
    let (_, _, hist) = warm_histogram(PREVIEW, 10_000_000);
    let mut prev = vec![0.0; PREVIEW.0 * PREVIEW.1];
    let mut buffer = vec![0u32; PREVIEW.0 * PREVIEW.1];
    bencher.bench_local(|| blend_and_colour(&hist, &mut prev, 0.9, POWER, &mut buffer));
}

/// What a parameter change costs before the first new frame can be drawn, including warm-up.
#[divan::bench]
fn parameter_change(bencher: divan::Bencher) {
    let (mut c, mut sampler, mut hist) = warm_histogram(PREVIEW, 10_000_000);
    let mut a = c.params()[0];
    bencher.bench_local(|| {
        a += 1e-3;
        c = c.with_param(0, a);
        sampler.reset();
        hist.clear();
        sampler.step(&c, 500_000, &mut hist, &View::default());
    });
}

#[divan::bench(sample_count = 5)]
fn write_png_a5(bencher: divan::Bencher) {
    let (_, _, hist) = warm_histogram(A5, A5.0 * A5.1);
    let path = std::env::temp_dir().join("attractors_bench.png");
    bencher.bench_local(|| write_png(&hist, POWER, &path).unwrap());
    let _ = std::fs::remove_file(path);
}

/// The formula alone on one core, with no histogram writes.
#[divan::bench]
fn formula_only_500k(bencher: divan::Bencher) {
    let c = CliffordAttractor::from_params(CliffordAttractor::DEFAULT_PARAMS);
    bencher.bench_local(|| {
        let (mut x, mut y) = (0.0, 0.0);
        for _ in 0..500_000 {
            (x, y) = c.next(divan::black_box(x), y);
        }
        (x, y)
    });
}

/// Full stepping of one trajectory on one core.
#[divan::bench]
fn step_500k_one_core(bencher: divan::Bencher) {
    let c = CliffordAttractor::from_params(CliffordAttractor::DEFAULT_PARAMS);
    let mut sampler = Sampler::with_walkers(1, CliffordAttractor::START);
    let mut hist = Histogram::new(PREVIEW.0, PREVIEW.1);
    sampler.step(&c, 0, &mut hist, &View::default());
    bencher.bench_local(|| {
        for _ in 0..10 {
            sampler.step(&c, 50_000, &mut hist, &View::default());
        }
    });
}

const WALKERS: [usize; 4] = [10, 20, 40, 80];

/// A steady frame with different walker counts: more walkers balance slow cores better.
#[divan::bench(args = WALKERS)]
fn step_500k_walkers(bencher: divan::Bencher, walkers: usize) {
    let c = CliffordAttractor::from_params(CliffordAttractor::DEFAULT_PARAMS);
    let mut sampler = Sampler::with_walkers(walkers, CliffordAttractor::START);
    let mut hist = Histogram::new(PREVIEW.0, PREVIEW.1);
    sampler.step(&c, 0, &mut hist, &View::default());
    bencher.bench_local(|| sampler.step(&c, 500_000, &mut hist, &View::default()));
}

/// A parameter change with different walker counts: more walkers mean more warm-up.
#[divan::bench(args = WALKERS)]
fn parameter_change_walkers(bencher: divan::Bencher, walkers: usize) {
    let mut c = CliffordAttractor::from_params(CliffordAttractor::DEFAULT_PARAMS);
    let mut sampler = Sampler::with_walkers(walkers, CliffordAttractor::START);
    let mut hist = Histogram::new(PREVIEW.0, PREVIEW.1);
    let mut a = c.params()[0];
    bencher.bench_local(|| {
        a += 1e-3;
        c = c.with_param(0, a);
        sampler.reset();
        hist.clear();
        sampler.step(&c, 500_000, &mut hist, &View::default());
    });
}
