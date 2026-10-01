//! How stepping scales with thread count, to find what stops it scaling.
//!
//! Every variant does one frame's worth of steps split evenly across `threads` independent
//! trajectories, differing only in what happens to each point.

use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;

use attractors::{Attractor, CliffordAttractor};

const STEPS: usize = 500_000;
const WIDTH: usize = 636;
const HEIGHT: usize = 900;
const THREADS: [usize; 5] = [1, 2, 4, 8, 10];

fn main() {
    divan::main();
}

fn pixel(c: &CliffordAttractor, x: f64, y: f64) -> Option<usize> {
    let (xmin, xmax, ymin, ymax) = c.bounds();
    let nx = (x - xmin) / (xmax - xmin);
    let ny = (y - ymin) / (ymax - ymin);
    if !((0.0..1.0).contains(&nx) && (0.0..1.0).contains(&ny)) {
        return None;
    }
    Some((nx * WIDTH as f64) as usize + (ny * HEIGHT as f64) as usize * WIDTH)
}

/// Warmed-up starting points, one per thread
fn starts(c: &CliffordAttractor, threads: usize) -> Vec<(f64, f64)> {
    (0..threads)
        .map(|i| {
            let (mut x, mut y) = (i as f64 * 1e-3, 0.0);
            for _ in 0..20_000 {
                (x, y) = c.next(x, y);
            }
            (x, y)
        })
        .collect()
}

#[divan::bench(args = THREADS)]
fn spawn_only(bencher: divan::Bencher, threads: usize) {
    bencher.bench_local(|| {
        thread::scope(|s| {
            for _ in 0..threads {
                s.spawn(|| divan::black_box(0));
            }
        })
    });
}

#[divan::bench(args = THREADS)]
fn formula_only(bencher: divan::Bencher, threads: usize) {
    let c = CliffordAttractor::new(CliffordAttractor::DEFAULT_PARAMS);
    let starts = starts(&c, threads);
    bencher.bench_local(|| {
        thread::scope(|s| {
            for &(x0, y0) in &starts {
                let c = &c;
                s.spawn(move || {
                    let (mut x, mut y) = (x0, y0);
                    for _ in 0..STEPS / threads {
                        (x, y) = c.next(divan::black_box(x), y);
                    }
                    (x, y)
                });
            }
        })
    });
}

#[divan::bench(args = THREADS)]
fn shared_atomic_counts(bencher: divan::Bencher, threads: usize) {
    let c = CliffordAttractor::new(CliffordAttractor::DEFAULT_PARAMS);
    let starts = starts(&c, threads);
    let counts: Vec<AtomicU32> = (0..WIDTH * HEIGHT).map(|_| AtomicU32::new(0)).collect();
    bencher.bench_local(|| {
        thread::scope(|s| {
            for &(x0, y0) in &starts {
                let (c, counts) = (&c, &counts);
                s.spawn(move || {
                    let (mut x, mut y) = (x0, y0);
                    for _ in 0..STEPS / threads {
                        (x, y) = c.next(x, y);
                        if let Some(i) = pixel(c, x, y) {
                            counts[i].fetch_add(1, Ordering::Relaxed);
                        }
                    }
                });
            }
        })
    });
}

#[divan::bench(args = THREADS)]
fn per_thread_counts_then_merge(bencher: divan::Bencher, threads: usize) {
    let c = CliffordAttractor::new(CliffordAttractor::DEFAULT_PARAMS);
    let starts = starts(&c, threads);
    let mut locals = vec![vec![0u32; WIDTH * HEIGHT]; threads];
    let mut total = vec![0u32; WIDTH * HEIGHT];
    bencher.bench_local(|| {
        thread::scope(|s| {
            for (&(x0, y0), local) in starts.iter().zip(&mut locals) {
                let c = &c;
                s.spawn(move || {
                    let (mut x, mut y) = (x0, y0);
                    for _ in 0..STEPS / threads {
                        (x, y) = c.next(x, y);
                        if let Some(i) = pixel(c, x, y) {
                            local[i] += 1;
                        }
                    }
                });
            }
        });
        // Merge in parallel by row bands, clearing locals for the next frame
        let band = total.len().div_ceil(threads);
        let mut local_bands: Vec<_> = locals.iter_mut().map(|l| l.chunks_mut(band)).collect();
        thread::scope(|s| {
            for out in total.chunks_mut(band) {
                let mut sources: Vec<&mut [u32]> =
                    local_bands.iter_mut().map(|b| b.next().unwrap()).collect();
                s.spawn(move || {
                    for src in &mut sources {
                        for (o, v) in out.iter_mut().zip(src.iter_mut()) {
                            *o += *v;
                            *v = 0;
                        }
                    }
                });
            }
        });
    });
}

/// Ten frames' worth in one burst, to see whether short bursts stop extra cores helping
#[divan::bench(args = THREADS, sample_count = 20)]
fn formula_only_5m(bencher: divan::Bencher, threads: usize) {
    let c = CliffordAttractor::new(CliffordAttractor::DEFAULT_PARAMS);
    let starts = starts(&c, threads);
    bencher.bench_local(|| {
        thread::scope(|s| {
            for &(x0, y0) in &starts {
                let c = &c;
                s.spawn(move || {
                    let (mut x, mut y) = (x0, y0);
                    for _ in 0..10 * STEPS / threads {
                        (x, y) = c.next(divan::black_box(x), y);
                    }
                    (x, y)
                });
            }
        })
    });
}

/// Same as formula_only_5m, but work is cut into many pieces taken from a shared counter
#[divan::bench(args = THREADS, sample_count = 20)]
fn formula_only_5m_work_queue(bencher: divan::Bencher, threads: usize) {
    const PIECES: usize = 400;
    let c = CliffordAttractor::new(CliffordAttractor::DEFAULT_PARAMS);
    let starts = starts(&c, threads);
    bencher.bench_local(|| {
        let next_piece = std::sync::atomic::AtomicUsize::new(0);
        thread::scope(|s| {
            for &(x0, y0) in &starts {
                let (c, next_piece) = (&c, &next_piece);
                s.spawn(move || {
                    let (mut x, mut y) = (x0, y0);
                    while next_piece.fetch_add(1, Ordering::Relaxed) < PIECES {
                        for _ in 0..10 * STEPS / PIECES {
                            (x, y) = c.next(divan::black_box(x), y);
                        }
                    }
                    (x, y)
                });
            }
        })
    });
}
