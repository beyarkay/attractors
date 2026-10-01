use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use std::thread;

use crate::attractors::Attractor;

/// Which part of the attractor's bounding box is drawn, and how much empty border surrounds it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    /// Fraction of the width/height left empty on each side.
    pub border: f64,
    pub hpan: f64,
    pub vpan: f64,
    /// 2.0 shows the middle half of the bounding box, 0.5 shows twice the bounding box.
    pub zoom: f64,
}

impl Default for View {
    fn default() -> Self {
        View {
            border: 0.05,
            hpan: 0.0,
            vpan: 0.0,
            zoom: 1.0,
        }
    }
}

/// How many times the attractor landed on each pixel of a `width` x `height` grid, row-major.
#[derive(Debug, Clone)]
pub struct Histogram {
    pub width: usize,
    pub height: usize,
    pub counts: Vec<u32>,
    pub max: u32,
    /// Every step taken into this histogram, including ones that landed outside the view.
    pub steps: usize,
}

impl Histogram {
    pub fn new(width: usize, height: usize) -> Self {
        Histogram {
            width,
            height,
            counts: vec![0; width * height],
            max: 0,
            steps: 0,
        }
    }

    pub fn clear(&mut self) {
        self.counts.fill(0);
        self.max = 0;
        self.steps = 0;
    }
}

/// Fewer steps than this run on the calling thread, where spawning threads would cost more
/// than it saves
const PARALLEL_MIN_STEPS: usize = 100_000;
/// Uncounted steps each walker takes after `Sampler::reset`, so slow transients aren't drawn
pub const WARMUP_STEPS: usize = 20_000;
/// More walkers than threads lets fast cores run more walkers than slow ones
const WALKERS_PER_THREAD: usize = 2;
/// Each thread counts into its own grid when all the grids fit in this many bytes, which avoids
/// cores contending on hot pixels. Bigger images (large saves) share one grid of atomic counters.
const PER_THREAD_COUNTS_MAX_BYTES: usize = 256 << 20;

pub fn num_threads() -> usize {
    thread::available_parallelism().map_or(1, |n| n.get())
}

/// Several independent trajectories ("walkers") that together sample an attractor.
///
/// Walkers keep their positions between calls to `step`, so once warmed up the result is the
/// same as adding together several long single-core runs. Each walker takes a fixed share of
/// every call's steps, so the output doesn't depend on thread scheduling. Threads take whole
/// walkers from a queue, so slower cores simply run fewer of them.
#[derive(Clone)]
pub struct Sampler {
    positions: Vec<(f64, f64)>,
    needs_warmup: bool,
    /// One grid per thread, kept zeroed between calls
    per_thread_counts: Vec<Vec<u32>>,
}

impl Default for Sampler {
    fn default() -> Self {
        Sampler::new(WALKERS_PER_THREAD * num_threads(), (0.0, 0.0))
    }
}

impl Sampler {
    pub fn new(walkers: usize, start: (f64, f64)) -> Self {
        assert!(walkers > 0);
        Sampler {
            positions: vec![start; walkers],
            needs_warmup: true,
            per_thread_counts: Vec::new(),
        }
    }

    /// Call after changing the attractor's parameters. Walkers restart, slightly nudged apart,
    /// from where the first walker was, and warm up again before anything is counted.
    pub fn reset(&mut self) {
        let start = self.positions[0];
        self.positions.fill(start);
        self.needs_warmup = true;
    }

    /// Take `num_steps` steps in total across all walkers, counting each point in `hist`.
    pub fn step<A: Attractor>(
        &mut self,
        attractor: &A,
        num_steps: usize,
        hist: &mut Histogram,
        view: &View,
    ) {
        let walkers = self.positions.len();
        let all_threads = num_threads().min(walkers);
        if self.needs_warmup {
            self.needs_warmup = false;
            let start = self.positions[0];
            run_walkers(
                &mut self.positions,
                &mut vec![(); all_threads],
                |i, pos, _| {
                    let (mut x, mut y) = (start.0 + i as f64 * 1e-3, start.1);
                    for _ in 0..WARMUP_STEPS {
                        (x, y) = attractor.next(x, y);
                    }
                    *pos = (x, y);
                },
            );
        }

        let mapping = PixelMapping::new(attractor.bounds(), view, hist.width, hist.height);
        let share = |i: usize| num_steps / walkers + usize::from(i < num_steps % walkers);
        let threads = if num_steps < PARALLEL_MIN_STEPS {
            1
        } else {
            all_threads
        };
        let cells = hist.counts.len();
        if threads == 1 {
            let mut state = [(hist.counts.as_mut_slice(), 0)];
            run_walkers(&mut self.positions, &mut state, |i, pos, (counts, max)| {
                *max = walk(attractor, &mapping, share(i), pos, &mut **counts).max(*max);
            });
            hist.max = hist.max.max(state[0].1);
        } else if threads * cells * size_of::<u32>() <= PER_THREAD_COUNTS_MAX_BYTES {
            let locals = &mut self.per_thread_counts;
            locals.resize_with(threads, Vec::new);
            for local in locals.iter_mut().filter(|l| l.len() != cells) {
                *local = vec![0; cells];
            }
            run_walkers(&mut self.positions, locals, |i, pos, local| {
                walk(attractor, &mapping, share(i), pos, local.as_mut_slice());
            });
            hist.max = hist.max.max(merge_into(&mut hist.counts, locals));
        } else {
            // SAFETY: AtomicU32 has the same in-memory representation as u32, and `hist.counts`
            // is exclusively borrowed for as long as `shared` is used.
            let shared: &[AtomicU32] =
                unsafe { &*(hist.counts.as_mut_slice() as *mut [u32] as *const [AtomicU32]) };
            let mut maxes = vec![0; threads];
            run_walkers(&mut self.positions, &mut maxes, |i, pos, max| {
                let mut counts = SharedCounts(shared);
                *max = walk(attractor, &mapping, share(i), pos, &mut counts).max(*max);
            });
            hist.max = maxes.into_iter().fold(hist.max, u32::max);
        }
        hist.steps += num_steps;
    }
}

trait Counts {
    /// Add one to the count at `idx`, returning the new count
    fn add(&mut self, idx: usize) -> u32;
}

impl Counts for [u32] {
    fn add(&mut self, idx: usize) -> u32 {
        self[idx] += 1;
        self[idx]
    }
}

struct SharedCounts<'a>(&'a [AtomicU32]);

impl Counts for SharedCounts<'_> {
    fn add(&mut self, idx: usize) -> u32 {
        self.0[idx].fetch_add(1, Ordering::Relaxed) + 1
    }
}

/// Advance one walker `steps` times, counting each point. Returns the largest count produced.
fn walk<A: Attractor, C: Counts + ?Sized>(
    attractor: &A,
    mapping: &PixelMapping,
    steps: usize,
    pos: &mut (f64, f64),
    counts: &mut C,
) -> u32 {
    let (mut x, mut y) = *pos;
    let mut max = 0;
    for _ in 0..steps {
        (x, y) = attractor.next(x, y);
        if let Some(idx) = mapping.index(x, y) {
            max = max.max(counts.add(idx));
        }
    }
    *pos = (x, y);
    max
}

/// Run `f(walker index, walker position, thread state)` for every walker, with one thread per
/// element of `per_thread`. Threads take walkers from a queue, so faster cores run more of them.
fn run_walkers<T, F>(positions: &mut [(f64, f64)], per_thread: &mut [T], f: F)
where
    T: Send,
    F: Fn(usize, &mut (f64, f64), &mut T) + Sync,
{
    if let [state] = per_thread {
        for (i, pos) in positions.iter_mut().enumerate() {
            f(i, pos, state);
        }
        return;
    }
    let queue = Mutex::new(positions.iter_mut().enumerate());
    thread::scope(|s| {
        for state in per_thread {
            let (queue, f) = (&queue, &f);
            s.spawn(move || loop {
                // Separate statement so the lock is released before walking
                let next = queue.lock().unwrap().next();
                let Some((i, pos)) = next else { break };
                f(i, pos, state);
            });
        }
    });
}

/// Add every per-thread grid into `total` and zero them, in parallel bands. Returns the largest
/// count in `total`.
fn merge_into(total: &mut [u32], locals: &mut [Vec<u32>]) -> u32 {
    let band = total.len().div_ceil(locals.len());
    let mut local_bands: Vec<_> = locals.iter_mut().map(|l| l.chunks_mut(band)).collect();
    thread::scope(|s| {
        let handles: Vec<_> = total
            .chunks_mut(band)
            .map(|out| {
                let sources: Vec<&mut [u32]> =
                    local_bands.iter_mut().map(|b| b.next().unwrap()).collect();
                s.spawn(move || {
                    for src in sources {
                        for (o, v) in out.iter_mut().zip(src) {
                            *o += *v;
                            *v = 0;
                        }
                    }
                    out.iter().copied().max().unwrap_or(0)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .fold(0, u32::max)
    })
}

/// Maps attractor coordinates to histogram indices for a given view.
#[derive(Clone, Copy)]
struct PixelMapping {
    zoomed_xmin: f64,
    zoomed_ymin: f64,
    xrange: f64,
    yrange: f64,
    x_scale: f64,
    y_scale: f64,
    x_offset: f64,
    y_offset: f64,
    width: usize,
    height: usize,
}

impl PixelMapping {
    fn new(bounds: (f64, f64, f64, f64), view: &View, width: usize, height: usize) -> Self {
        let (xmin, xmax, ymin, ymax) = bounds;
        let xrange = (xmax - xmin) / view.zoom;
        let yrange = (ymax - ymin) / view.zoom;
        let non_border_frac = 1.0 - 2.0 * view.border;
        PixelMapping {
            zoomed_xmin: xmin + (xmax - xmin) * 0.5 + view.hpan - xrange * 0.5,
            zoomed_ymin: ymin + (ymax - ymin) * 0.5 + view.vpan - yrange * 0.5,
            xrange,
            yrange,
            x_scale: width as f64 * non_border_frac,
            y_scale: height as f64 * non_border_frac,
            x_offset: width as f64 * view.border,
            y_offset: height as f64 * view.border,
            width,
            height,
        }
    }

    #[inline(always)]
    fn index(&self, x: f64, y: f64) -> Option<usize> {
        let normed_x = (x - self.zoomed_xmin) / self.xrange;
        let normed_y = (y - self.zoomed_ymin) / self.yrange;
        // Written so that NaN is also skipped
        if !((0.0..=1.0).contains(&normed_x) && (0.0..=1.0).contains(&normed_y)) {
            return None;
        }
        let px = ((normed_x * self.x_scale + self.x_offset) as usize).min(self.width - 1);
        let py = ((normed_y * self.y_scale + self.y_offset) as usize).min(self.height - 1);
        Some(px + py * self.width)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attractors::CliffordAttractor;

    const PARAMS: [f64; 4] = [-1.4, 1.6, 1.0, 0.7];

    #[test]
    fn step_takes_exactly_n_steps() {
        let c = CliffordAttractor::new(PARAMS);
        let mut sampler = Sampler::default();
        let mut hist = Histogram::new(10, 10);
        sampler.step(&c, 1, &mut hist, &View::default());
        assert_eq!(hist.steps, 1);
        assert_eq!(hist.counts.iter().sum::<u32>(), 1);
        sampler.step(&c, 1000, &mut hist, &View::default());
        assert_eq!(hist.steps, 1001);
        assert_eq!(hist.counts.iter().map(|&n| n as usize).sum::<usize>(), 1001);
    }

    #[test]
    fn point_on_the_far_edge_with_no_border_stays_in_bounds() {
        // Always lands on (1, 1), and panning by -1 puts that exactly on the far edge
        let c = CliffordAttractor::new([0.0, 0.0, 1.0, 1.0]);
        let mut hist = Histogram::new(7, 5);
        let view = View {
            border: 0.0,
            hpan: -1.0,
            vpan: -1.0,
            zoom: 1.0,
        };
        Sampler::default().step(&c, 10, &mut hist, &view);
        assert_eq!(hist.counts[7 * 5 - 1], 10);
    }

    #[test]
    fn parallel_step_counts_every_point_and_tracks_max() {
        let c = CliffordAttractor::new(PARAMS);
        let mut sampler = Sampler::default();
        let mut hist = Histogram::new(64, 64);
        let n = PARALLEL_MIN_STEPS * 3 + 7;
        sampler.step(&c, n, &mut hist, &View::default());
        sampler.step(&c, n, &mut hist, &View::default());
        assert_eq!(hist.steps, 2 * n);
        assert_eq!(
            hist.counts.iter().map(|&n| n as usize).sum::<usize>(),
            2 * n
        );
        assert_eq!(Some(&hist.max), hist.counts.iter().max());
    }

    #[test]
    fn parallel_step_is_deterministic() {
        let c = CliffordAttractor::new(PARAMS);
        let render = || {
            let mut sampler = Sampler::default();
            let mut hist = Histogram::new(64, 64);
            for _ in 0..3 {
                sampler.step(&c, PARALLEL_MIN_STEPS * 2, &mut hist, &View::default());
            }
            hist.counts
        };
        assert_eq!(render(), render());
    }

    #[test]
    fn view_far_away_counts_nothing() {
        let c = CliffordAttractor::new(PARAMS);
        let mut hist = Histogram::new(10, 10);
        let view = View {
            hpan: 1000.0,
            ..View::default()
        };
        Sampler::default().step(&c, 1000, &mut hist, &view);
        assert_eq!(hist.max, 0);
        assert_eq!(hist.steps, 1000);
    }

    #[test]
    fn histogram_too_big_for_per_thread_grids_counts_every_point() {
        let threads = num_threads();
        let side = ((PER_THREAD_COUNTS_MAX_BYTES / 4 / threads) as f64).sqrt() as usize + 1;
        let c = CliffordAttractor::new(PARAMS);
        let mut sampler = Sampler::default();
        let mut hist = Histogram::new(side, side);
        let n = PARALLEL_MIN_STEPS * 2;
        sampler.step(&c, n, &mut hist, &View::default());
        assert_eq!(hist.counts.iter().map(|&n| n as usize).sum::<usize>(), n);
        assert_eq!(Some(&hist.max), hist.counts.iter().max());
    }
}
