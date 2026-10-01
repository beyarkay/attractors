use attractors::{Attractor, CliffordAttractor};

const PREVIEW: (usize, usize) = (636, 900);
const PARAMS: [f64; 4] = [-1.4, 1.6, 1.0, 0.7];

fn main() {
    divan::main();
}

#[divan::bench]
fn step_500k(bencher: divan::Bencher) {
    bencher
        .with_inputs(|| CliffordAttractor::new(PARAMS.to_vec()))
        .bench_local_values(|mut c| {
            c.step(500_000);
            c
        });
}

/// What the interactive loop does every frame once the attractor has 10 million points.
#[divan::bench(sample_count = 10)]
fn preview_densities_10m(bencher: divan::Bencher) {
    let mut c = CliffordAttractor::new(PARAMS.to_vec());
    c.step(10_000_000);
    bencher.bench_local(|| c.get_densities_with_border(PREVIEW.0, PREVIEW.1, 0.05, 0.0, 0.0, 1.0));
}
