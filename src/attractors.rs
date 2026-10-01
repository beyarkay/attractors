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

    /// Fill `out` with each pixel's count divided by the maximum count, so values are in [0, 1].
    /// An empty histogram gives all zeros.
    pub fn normalise_into(&self, out: &mut [f64]) {
        assert_eq!(out.len(), self.counts.len());
        if self.max == 0 {
            out.fill(0.0);
            return;
        }
        let inv_max = 1.0 / self.max as f64;
        for (o, &c) in out.iter_mut().zip(&self.counts) {
            *o = c as f64 * inv_max;
        }
    }
}

pub trait Attractor {
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

    /// Take `num_steps` steps from the current position, counting each point in `hist`.
    fn step(&mut self, num_steps: usize, hist: &mut Histogram, view: &View) {
        let (xmin, xmax, ymin, ymax) = self.bounds();
        let xrange = (xmax - xmin) / view.zoom;
        let yrange = (ymax - ymin) / view.zoom;
        let zoomed_xmin = xmin + (xmax - xmin) * 0.5 + view.hpan - xrange * 0.5;
        let zoomed_ymin = ymin + (ymax - ymin) * 0.5 + view.vpan - yrange * 0.5;

        let (w, h) = (hist.width, hist.height);
        let non_border_frac = 1.0 - 2.0 * view.border;
        let x_scale = w as f64 * non_border_frac;
        let y_scale = h as f64 * non_border_frac;
        let x_offset = w as f64 * view.border;
        let y_offset = h as f64 * view.border;

        let (mut x, mut y) = self.position();
        let mut max = hist.max;
        for _ in 0..num_steps {
            (x, y) = self.next(x, y);
            let normed_x = (x - zoomed_xmin) / xrange;
            let normed_y = (y - zoomed_ymin) / yrange;
            // Written so that NaN is also skipped
            if !((0.0..=1.0).contains(&normed_x) && (0.0..=1.0).contains(&normed_y)) {
                continue;
            }
            let px = ((normed_x * x_scale + x_offset) as usize).min(w - 1);
            let py = ((normed_y * y_scale + y_offset) as usize).min(h - 1);
            let count = &mut hist.counts[px + py * w];
            *count += 1;
            max = max.max(*count);
        }
        hist.max = max;
        hist.steps += num_steps;
        self.set_position(x, y);
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
    fn empty_histogram_normalises_to_zero() {
        let hist = Histogram::new(4, 4);
        let mut out = vec![f64::NAN; 16];
        hist.normalise_into(&mut out);
        assert!(out.iter().all(|&v| v == 0.0));
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
