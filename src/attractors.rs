/// A trait to collect common functions used in defining strange attractors.
///
/// Examples of strange attractors:
/// - [Clifford Attractors](http://paulbourke.net/fractals/clifford/)
/// - [De Jong Attractors](http://paulbourke.net/fractals/peterdejong/)
/// - [Lyapunov Exponent Attractors](http://paulbourke.net/fractals/lyapunov/)
/// - [Sprott Polynomial Attractors](http://paulbourke.net/fractals/sprott/)
/// - [Juan Attractors](http://paulbourke.net/fractals/juan2/)
/// - [Den Tsucs Attractors](http://paulbourke.net/fractals/tsucs/)
/// - [Sánchez 'Bad Hairday' Attractors](http://paulbourke.net/fractals/2dmap/), also see
///   [here](https://www.r-bloggers.com/2019/10/strange-attractors-an-r-experiment-about-maths-recursivity-and-creative-coding/)
/// - [Arneodo Attractors](http://paulbourke.net/fractals/arneodo/)
/// - [Catrián Attractors](http://paulbourke.net/fractals/JuanCatrian/)
/// - [Burke-Shaw Attractors](http://paulbourke.net/fractals/burkeshaw/)
/// - [Yu-Wang Attractors](http://paulbourke.net/fractals/yuwang/)
use std::fmt::{self, Display};
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;

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

pub trait Attractor: Sync {
    /// A name for the attractor matching [a-zA-Z]+
    const NAME: &'static str;
    /// Parameters that give a nice-looking attractor
    const DEFAULT_PARAMS: [f64; 4];

    fn new(params: [f64; 4]) -> Self;

    fn params(&self) -> [f64; 4];

    /// `None` leaves that parameter unchanged.
    fn set_params(&mut self, params: [Option<f64>; 4]);

    /// A box `(xmin, xmax, ymin, ymax)` guaranteed to contain every point of the attractor.
    fn bounds(&self) -> (f64, f64, f64, f64);

    fn position(&self) -> (f64, f64);

    fn set_position(&mut self, x: f64, y: f64);

    /// One application of the attractor's formula.
    fn next(&self, x: f64, y: f64) -> (f64, f64);

    /// Take `num_steps` steps, counting each point in `hist`.
    ///
    /// Large step counts are split across all cores. Each extra thread follows its own
    /// trajectory, starting from a slightly nudged copy of the current position.
    fn step(&mut self, num_steps: usize, hist: &mut Histogram, view: &View)
    where
        Self: Sized,
    {
        let mapping = PixelMapping::new(self.bounds(), view, hist.width, hist.height);
        let threads = if num_steps < PARALLEL_MIN_STEPS {
            1
        } else {
            num_threads()
        };
        // SAFETY: AtomicU32 has the same in-memory representation as u32, and `hist.counts` is
        // exclusively borrowed for as long as `counts` is used.
        let counts: &[AtomicU32] =
            unsafe { &*(hist.counts.as_mut_slice() as *mut [u32] as *const [AtomicU32]) };
        let start = self.position();
        let this = &*self;
        let walk = |thread_idx: usize| {
            let n = num_steps / threads + usize::from(thread_idx < num_steps % threads);
            let (mut x, mut y) = (start.0 + thread_idx as f64 * 1e-3, start.1);
            if thread_idx > 0 {
                for _ in 0..WARMUP_STEPS {
                    (x, y) = this.next(x, y);
                }
            }
            let mut max = 0;
            for _ in 0..n {
                (x, y) = this.next(x, y);
                if let Some(idx) = mapping.index(x, y) {
                    max = max.max(counts[idx].fetch_add(1, Ordering::Relaxed) + 1);
                }
            }
            ((x, y), max)
        };
        let results: Vec<((f64, f64), u32)> = if threads == 1 {
            vec![walk(0)]
        } else {
            thread::scope(|s| {
                let handles: Vec<_> = (0..threads).map(|i| s.spawn(move || walk(i))).collect();
                handles.into_iter().map(|h| h.join().unwrap()).collect()
            })
        };
        // Counts only increase, so the largest value any increment produced is the new max
        hist.max = results.iter().map(|r| r.1).fold(hist.max, u32::max);
        hist.steps += num_steps;
        let (x, y) = results[0].0;
        self.set_position(x, y);
    }
}

/// Fewer steps than this run on one thread, where spawning threads would cost more than it saves
const PARALLEL_MIN_STEPS: usize = 100_000;
/// Uncounted steps for each extra thread, so its trajectory has diverged from thread 0's
const WARMUP_STEPS: usize = 100;

pub fn num_threads() -> usize {
    thread::available_parallelism().map_or(1, |n| n.get())
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

/// Parse a line like `a=1.0,b=-2.0,c=0.5,d=0.1`. Extra comma-separated fields are ignored.
pub fn parse_params(line: &str) -> Option<[f64; 4]> {
    let mut fields = line.split(',');
    let mut params = [0.0; 4];
    for (param, name) in params.iter_mut().zip(["a", "b", "c", "d"]) {
        let (key, value) = fields.next()?.split_once('=')?;
        if key.trim() != name {
            return None;
        }
        *param = value.trim().parse().ok()?;
    }
    Some(params)
}

pub fn format_params(params: [f64; 4]) -> String {
    format!(
        "a={},b={},c={},d={}",
        params[0], params[1], params[2], params[3]
    )
}

/// A Clifford Attractor, as discovered by [Clifford A
/// Pickover](https://en.wikipedia.org/wiki/Clifford_A._Pickover)
///
/// ```text
/// x_new = sin(a * y) + c * cos(a * x)
/// y_new = sin(b * x) + d * cos(b * y)
/// ```
#[derive(Debug, Clone)]
pub struct CliffordAttractor {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub x: f64,
    pub y: f64,
}

impl Attractor for CliffordAttractor {
    const NAME: &'static str = "clifford";
    const DEFAULT_PARAMS: [f64; 4] = [-1.4, 1.6, 1.0, 0.7];

    fn new(params: [f64; 4]) -> Self {
        let [a, b, c, d] = params;
        CliffordAttractor {
            a,
            b,
            c,
            d,
            x: 0.0,
            y: 0.0,
        }
    }

    fn params(&self) -> [f64; 4] {
        [self.a, self.b, self.c, self.d]
    }

    fn set_params(&mut self, params: [Option<f64>; 4]) {
        let [a, b, c, d] = params;
        self.a = a.unwrap_or(self.a);
        self.b = b.unwrap_or(self.b);
        self.c = c.unwrap_or(self.c);
        self.d = d.unwrap_or(self.d);
    }

    fn bounds(&self) -> (f64, f64, f64, f64) {
        // |sin| <= 1 and |c * cos| <= |c|
        (
            -1.0 - self.c.abs(),
            1.0 + self.c.abs(),
            -1.0 - self.d.abs(),
            1.0 + self.d.abs(),
        )
    }

    fn position(&self) -> (f64, f64) {
        (self.x, self.y)
    }

    fn set_position(&mut self, x: f64, y: f64) {
        self.x = x;
        self.y = y;
    }

    #[inline(always)]
    fn next(&self, x: f64, y: f64) -> (f64, f64) {
        (
            (self.a * y).sin() + self.c * (self.a * x).cos(),
            (self.b * x).sin() + self.d * (self.b * y).cos(),
        )
    }
}

impl Display for CliffordAttractor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Clifford Attractor:\n  x_new = sin({a:+.4} * y) + {c:+.4} * cos({a:+.4} * x);\n  y_new = sin({b:+.4} * x) + {d:+.4} * cos({b:+.4} * y)",
        a=self.a,
        b=self.b,
        c=self.c,
        d=self.d,
        )
    }
}

/// [De Jong Attractors](http://paulbourke.net/fractals/peterdejong/)
///
/// ```text
/// x_new = sin(a * y) - cos(b * x)
/// y_new = sin(c * x) - cos(d * y)
/// ```
#[derive(Debug, Clone)]
pub struct DeJongAttractor {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub x: f64,
    pub y: f64,
}

impl Attractor for DeJongAttractor {
    const NAME: &'static str = "dejong";
    const DEFAULT_PARAMS: [f64; 4] = [-2.0, -2.0, -1.2, 2.0];

    fn new(params: [f64; 4]) -> Self {
        let [a, b, c, d] = params;
        DeJongAttractor {
            a,
            b,
            c,
            d,
            x: 0.0,
            y: 0.0,
        }
    }

    fn params(&self) -> [f64; 4] {
        [self.a, self.b, self.c, self.d]
    }

    fn set_params(&mut self, params: [Option<f64>; 4]) {
        let [a, b, c, d] = params;
        self.a = a.unwrap_or(self.a);
        self.b = b.unwrap_or(self.b);
        self.c = c.unwrap_or(self.c);
        self.d = d.unwrap_or(self.d);
    }

    fn bounds(&self) -> (f64, f64, f64, f64) {
        (-2.0, 2.0, -2.0, 2.0)
    }

    fn position(&self) -> (f64, f64) {
        (self.x, self.y)
    }

    fn set_position(&mut self, x: f64, y: f64) {
        self.x = x;
        self.y = y;
    }

    #[inline(always)]
    fn next(&self, x: f64, y: f64) -> (f64, f64) {
        (
            (self.a * y).sin() - (self.b * x).cos(),
            (self.c * x).sin() - (self.d * y).cos(),
        )
    }
}

impl Display for DeJongAttractor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "De Jong Attractor:\n  x_new = sin({a:+.4} * y) - cos({b:+.4} * x);\n  y_new = sin({c:+.4} * x) - cos({d:+.4} * y)",
        a=self.a,
        b=self.b,
        c=self.c,
        d=self.d,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PARAMS: [f64; 4] = [-1.4, 1.6, 1.0, 0.7];

    #[test]
    fn step_takes_exactly_n_steps() {
        let mut c = CliffordAttractor::new(PARAMS);
        let mut hist = Histogram::new(10, 10);
        c.step(1, &mut hist, &View::default());
        assert_eq!(hist.steps, 1);
        assert_eq!(c.position(), CliffordAttractor::new(PARAMS).next(0.0, 0.0));
        c.step(1000, &mut hist, &View::default());
        assert_eq!(hist.steps, 1001);
        assert_eq!(hist.counts.iter().map(|&n| n as usize).sum::<usize>(), 1001);
    }

    #[test]
    fn clifford_y_uses_previous_x() {
        let c = CliffordAttractor::new(PARAMS);
        let (x, y) = (0.3, -0.2);
        let (nx, ny) = c.next(x, y);
        assert_eq!(nx, (-1.4 * y).sin() + 1.0 * (-1.4 * x).cos());
        assert_eq!(ny, (1.6 * x).sin() + 0.7 * (1.6 * y).cos());
    }

    #[test]
    fn point_on_the_far_edge_with_no_border_stays_in_bounds() {
        // Always lands on (1, 1), and panning by -1 puts that exactly on the far edge
        let mut c = CliffordAttractor::new([0.0, 0.0, 1.0, 1.0]);
        let mut hist = Histogram::new(7, 5);
        let view = View {
            border: 0.0,
            hpan: -1.0,
            vpan: -1.0,
            zoom: 1.0,
        };
        c.step(10, &mut hist, &view);
        assert_eq!(hist.counts[7 * 5 - 1], 10);
    }

    #[test]
    fn parallel_step_counts_every_point_and_tracks_max() {
        let mut c = CliffordAttractor::new(PARAMS);
        let mut hist = Histogram::new(64, 64);
        let n = PARALLEL_MIN_STEPS * 3 + 7;
        c.step(n, &mut hist, &View::default());
        c.step(n, &mut hist, &View::default());
        assert_eq!(hist.steps, 2 * n);
        assert_eq!(
            hist.counts.iter().map(|&n| n as usize).sum::<usize>(),
            2 * n
        );
        assert_eq!(Some(&hist.max), hist.counts.iter().max());
    }

    #[test]
    fn view_far_away_counts_nothing() {
        let mut c = CliffordAttractor::new(PARAMS);
        let mut hist = Histogram::new(10, 10);
        let view = View {
            hpan: 1000.0,
            ..View::default()
        };
        c.step(1000, &mut hist, &view);
        assert_eq!(hist.max, 0);
        assert_eq!(hist.steps, 1000);
    }

    #[test]
    fn parse_params_round_trips() {
        let p = [1.378_6, -0.83, 1.004_6, -2.43];
        assert_eq!(parse_params(&format_params(p)), Some(p));
        assert_eq!(
            parse_params("a=1,b=2,c=3,d=4,iters=10"),
            Some([1.0, 2.0, 3.0, 4.0])
        );
    }

    #[test]
    fn parse_params_rejects_malformed_lines() {
        for line in ["", "a", "a=1,b=2,c=3", "a=1,b=2,c=3,e=4", "a=1,b=x,c=3,d=4"] {
            assert_eq!(parse_params(line), None, "{line:?}");
        }
    }
}
