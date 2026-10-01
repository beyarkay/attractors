//! Strange attractors. To add one: write a struct holding its parameters, implement
//! `Attractor` for it, and add it to the `attractors!` list in `main.rs`.
//!
//! Examples of strange attractors:
//! - [Clifford Attractors](http://paulbourke.net/fractals/clifford/)
//! - [De Jong Attractors](http://paulbourke.net/fractals/peterdejong/)
//! - [Lyapunov Exponent Attractors](http://paulbourke.net/fractals/lyapunov/)
//! - [Sprott Polynomial Attractors](http://paulbourke.net/fractals/sprott/)
//! - [Juan Attractors](http://paulbourke.net/fractals/juan2/)
//! - [Den Tsucs Attractors](http://paulbourke.net/fractals/tsucs/)
//! - [Sánchez 'Bad Hairday' Attractors](http://paulbourke.net/fractals/2dmap/), also see
//!   [here](https://www.r-bloggers.com/2019/10/strange-attractors-an-r-experiment-about-maths-recursivity-and-creative-coding/)
//! - [Arneodo Attractors](http://paulbourke.net/fractals/arneodo/)
//! - [Catrián Attractors](http://paulbourke.net/fractals/JuanCatrian/)
//! - [Burke-Shaw Attractors](http://paulbourke.net/fractals/burkeshaw/)
//! - [Yu-Wang Attractors](http://paulbourke.net/fractals/yuwang/)

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub xmin: f64,
    pub xmax: f64,
    pub ymin: f64,
    pub ymax: f64,
}

pub trait Attractor: Clone + Send + Sync + 'static {
    /// Lower case name matching [a-z]+, used on the command line and for the cache directory
    const NAME: &'static str;
    /// Printed when the attractor is selected
    const FORMULA: &'static str;
    const PARAM_NAMES: &'static [&'static str];
    /// Parameters that give a nice-looking attractor, one per entry of `PARAM_NAMES`
    const DEFAULT_PARAMS: &'static [f64];
    /// Range shown for every parameter on the map, and used for random jumps
    const PARAM_RANGE: (f64, f64);
    /// Where walkers start. Must not be a fixed point of the formula.
    const START: (f64, f64) = (0.0, 0.0);
    /// How much each key press changes a parameter, to begin with
    const STEP: f64 = 0.1;

    /// `params` has one value per entry of `PARAM_NAMES`
    fn from_params(params: &[f64]) -> Self;

    fn params(&self) -> &[f64];

    /// A box guaranteed to contain every point of the attractor, if one is known. Otherwise the
    /// sampler estimates one from its warm-up points.
    fn bounds(&self) -> Option<Bounds> {
        None
    }

    /// One application of the attractor's formula.
    fn next(&self, x: f64, y: f64) -> (f64, f64);

    fn with_param(&self, idx: usize, value: f64) -> Self {
        let mut params = self.params().to_vec();
        params[idx] = value;
        Self::from_params(&params)
    }
}

/// Parse a line like `a=1.0,b=-2.0` whose keys are exactly `names`, in order. Extra
/// comma-separated fields after those are ignored.
pub fn parse_params(line: &str, names: &[&str]) -> Option<Vec<f64>> {
    let mut fields = line.split(',');
    names
        .iter()
        .map(|&name| {
            let (key, value) = fields.next()?.split_once('=')?;
            if key.trim() != name {
                return None;
            }
            value.trim().parse().ok()
        })
        .collect()
}

pub fn format_params(names: &[&str], params: &[f64]) -> String {
    names
        .iter()
        .zip(params)
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join(",")
}

/// Average exponential rate at which nearby trajectories separate. Positive means chaotic,
/// negative means the trajectory settles onto a fixed point or cycle, and NaN means it escaped
/// to infinity.
pub fn lyapunov_exponent<A: Attractor>(attractor: &A, steps: usize) -> f64 {
    const SEPARATION: f64 = 1e-8;
    const ESCAPED: f64 = 1e6;
    let (mut x, mut y) = A::START;
    for _ in 0..1_000 {
        (x, y) = attractor.next(x, y);
    }
    let (mut xe, mut ye) = (x + SEPARATION, y);
    let mut sum = 0.0;
    for _ in 0..steps {
        (x, y) = attractor.next(x, y);
        (xe, ye) = attractor.next(xe, ye);
        if !(x.abs() < ESCAPED && y.abs() < ESCAPED) {
            return f64::NAN;
        }
        let (dx, dy) = (xe - x, ye - y);
        let d = dx.hypot(dy);
        if d == 0.0 {
            return f64::NEG_INFINITY;
        }
        sum += (d / SEPARATION).ln();
        xe = x + dx * SEPARATION / d;
        ye = y + dy * SEPARATION / d;
    }
    sum / steps as f64
}

/// [Clifford A. Pickover](https://en.wikipedia.org/wiki/Clifford_A._Pickover)'s attractor
#[derive(Debug, Clone)]
pub struct CliffordAttractor {
    p: [f64; 4],
}

impl Attractor for CliffordAttractor {
    const NAME: &'static str = "clifford";
    const FORMULA: &'static str = "x' = sin(a y) + c cos(a x)\ny' = sin(b x) + d cos(b y)";
    const PARAM_NAMES: &'static [&'static str] = &["a", "b", "c", "d"];
    const DEFAULT_PARAMS: &'static [f64] = &[-1.4, 1.6, 1.0, 0.7];
    const PARAM_RANGE: (f64, f64) = (-5.0, 5.0);

    fn from_params(params: &[f64]) -> Self {
        CliffordAttractor {
            p: params.try_into().expect("4 parameters"),
        }
    }

    fn params(&self) -> &[f64] {
        &self.p
    }

    fn bounds(&self) -> Option<Bounds> {
        // |sin| <= 1 and |c * cos| <= |c|
        let [_, _, c, d] = self.p;
        Some(Bounds {
            xmin: -1.0 - c.abs(),
            xmax: 1.0 + c.abs(),
            ymin: -1.0 - d.abs(),
            ymax: 1.0 + d.abs(),
        })
    }

    #[inline(always)]
    fn next(&self, x: f64, y: f64) -> (f64, f64) {
        let [a, b, c, d] = self.p;
        (
            (a * y).sin() + c * (a * x).cos(),
            (b * x).sin() + d * (b * y).cos(),
        )
    }
}

/// [Peter de Jong](http://paulbourke.net/fractals/peterdejong/)'s attractor
#[derive(Debug, Clone)]
pub struct DeJongAttractor {
    p: [f64; 4],
}

impl Attractor for DeJongAttractor {
    const NAME: &'static str = "dejong";
    const FORMULA: &'static str = "x' = sin(a y) - cos(b x)\ny' = sin(c x) - cos(d y)";
    const PARAM_NAMES: &'static [&'static str] = &["a", "b", "c", "d"];
    const DEFAULT_PARAMS: &'static [f64] = &[-2.0, -2.0, -1.2, 2.0];
    const PARAM_RANGE: (f64, f64) = (-4.0, 4.0);

    fn from_params(params: &[f64]) -> Self {
        DeJongAttractor {
            p: params.try_into().expect("4 parameters"),
        }
    }

    fn params(&self) -> &[f64] {
        &self.p
    }

    fn bounds(&self) -> Option<Bounds> {
        Some(Bounds {
            xmin: -2.0,
            xmax: 2.0,
            ymin: -2.0,
            ymax: 2.0,
        })
    }

    #[inline(always)]
    fn next(&self, x: f64, y: f64) -> (f64, f64) {
        let [a, b, c, d] = self.p;
        ((a * y).sin() - (b * x).cos(), (c * x).sin() - (d * y).cos())
    }
}

/// [The King's Dream](https://nathanselikoff.com/training/tutorial-strange-attractors-in-c-and-opengl)
#[derive(Debug, Clone)]
pub struct KingsDreamAttractor {
    p: [f64; 4],
}

impl Attractor for KingsDreamAttractor {
    const NAME: &'static str = "kingsdream";
    const FORMULA: &'static str = "x' = sin(b y) + c sin(b x)\ny' = sin(a x) + d sin(a y)";
    const PARAM_NAMES: &'static [&'static str] = &["a", "b", "c", "d"];
    const DEFAULT_PARAMS: &'static [f64] = &[-0.966918, 2.879879, 0.765145, 0.744728];
    const PARAM_RANGE: (f64, f64) = (-3.0, 3.0);
    const START: (f64, f64) = (0.1, 0.1);

    fn from_params(params: &[f64]) -> Self {
        KingsDreamAttractor {
            p: params.try_into().expect("4 parameters"),
        }
    }

    fn params(&self) -> &[f64] {
        &self.p
    }

    fn bounds(&self) -> Option<Bounds> {
        let [_, _, c, d] = self.p;
        Some(Bounds {
            xmin: -1.0 - c.abs(),
            xmax: 1.0 + c.abs(),
            ymin: -1.0 - d.abs(),
            ymax: 1.0 + d.abs(),
        })
    }

    #[inline(always)]
    fn next(&self, x: f64, y: f64) -> (f64, f64) {
        let [a, b, c, d] = self.p;
        (
            (b * y).sin() + c * (b * x).sin(),
            (a * x).sin() + d * (a * y).sin(),
        )
    }
}

/// [Michel Hénon](https://en.wikipedia.org/wiki/H%C3%A9non_map)'s map
#[derive(Debug, Clone)]
pub struct HenonAttractor {
    p: [f64; 2],
}

impl Attractor for HenonAttractor {
    const NAME: &'static str = "henon";
    const FORMULA: &'static str = "x' = 1 - a x^2 + y\ny' = b x";
    const PARAM_NAMES: &'static [&'static str] = &["a", "b"];
    const DEFAULT_PARAMS: &'static [f64] = &[1.4, 0.3];
    const PARAM_RANGE: (f64, f64) = (-2.0, 2.0);

    fn from_params(params: &[f64]) -> Self {
        HenonAttractor {
            p: params.try_into().expect("2 parameters"),
        }
    }

    fn params(&self) -> &[f64] {
        &self.p
    }

    #[inline(always)]
    fn next(&self, x: f64, y: f64) -> (f64, f64) {
        let [a, b] = self.p;
        (1.0 - a * x * x + y, b * x)
    }
}

/// The [Tinkerbell map](https://en.wikipedia.org/wiki/Tinkerbell_map)
#[derive(Debug, Clone)]
pub struct TinkerbellAttractor {
    p: [f64; 4],
}

impl Attractor for TinkerbellAttractor {
    const NAME: &'static str = "tinkerbell";
    const FORMULA: &'static str = "x' = x^2 - y^2 + a x + b y\ny' = 2 x y + c x + d y";
    const PARAM_NAMES: &'static [&'static str] = &["a", "b", "c", "d"];
    const DEFAULT_PARAMS: &'static [f64] = &[0.9, -0.6013, 2.0, 0.5];
    const PARAM_RANGE: (f64, f64) = (-3.0, 3.0);
    const START: (f64, f64) = (-0.72, -0.64);
    /// Steps of 0.1 in a or b send it to infinity
    const STEP: f64 = 0.01;

    fn from_params(params: &[f64]) -> Self {
        TinkerbellAttractor {
            p: params.try_into().expect("4 parameters"),
        }
    }

    fn params(&self) -> &[f64] {
        &self.p
    }

    #[inline(always)]
    fn next(&self, x: f64, y: f64) -> (f64, f64) {
        let [a, b, c, d] = self.p;
        (x * x - y * y + a * x + b * y, 2.0 * x * y + c * x + d * y)
    }
}

/// [Julien Sprott](http://paulbourke.net/fractals/sprott/)'s general quadratic map
#[derive(Debug, Clone)]
pub struct SprottQuadraticAttractor {
    p: [f64; 12],
}

impl Attractor for SprottQuadraticAttractor {
    const NAME: &'static str = "sprott";
    const FORMULA: &'static str = "x' = a1 + a2 x + a3 x^2 + a4 x y + a5 y + a6 y^2\n\
                                   y' = a7 + a8 x + a9 x^2 + a10 x y + a11 y + a12 y^2";
    const PARAM_NAMES: &'static [&'static str] = &[
        "a1", "a2", "a3", "a4", "a5", "a6", "a7", "a8", "a9", "a10", "a11", "a12",
    ];
    const DEFAULT_PARAMS: &'static [f64] = &[
        0.2, -0.8, -0.3, -0.6, -0.8, -0.8, 0.8, 0.0, -1.2, 0.2, 0.4, 0.3,
    ];
    const PARAM_RANGE: (f64, f64) = (-1.2, 1.2);
    const START: (f64, f64) = (0.05, 0.05);

    fn from_params(params: &[f64]) -> Self {
        SprottQuadraticAttractor {
            p: params.try_into().expect("12 parameters"),
        }
    }

    fn params(&self) -> &[f64] {
        &self.p
    }

    #[inline(always)]
    fn next(&self, x: f64, y: f64) -> (f64, f64) {
        let [a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12] = self.p;
        (
            a1 + a2 * x + a3 * x * x + a4 * x * y + a5 * y + a6 * y * y,
            a7 + a8 * x + a9 * x * x + a10 * x * y + a11 * y + a12 * y * y,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_defaults<A: Attractor>() {
        assert_eq!(A::DEFAULT_PARAMS.len(), A::PARAM_NAMES.len(), "{}", A::NAME);
        let a = A::from_params(A::DEFAULT_PARAMS);
        assert_eq!(a.params(), A::DEFAULT_PARAMS, "{}", A::NAME);
        let lyapunov = lyapunov_exponent(&a, 20_000);
        assert!(
            lyapunov > 0.0,
            "{} defaults aren't chaotic: {lyapunov}",
            A::NAME
        );
    }

    #[test]
    fn default_params_are_well_formed_and_chaotic() {
        check_defaults::<CliffordAttractor>();
        check_defaults::<DeJongAttractor>();
        check_defaults::<KingsDreamAttractor>();
        check_defaults::<HenonAttractor>();
        check_defaults::<TinkerbellAttractor>();
        check_defaults::<SprottQuadraticAttractor>();
    }

    #[test]
    fn clifford_y_uses_previous_x() {
        let c = CliffordAttractor::from_params(&[-1.4, 1.6, 1.0, 0.7]);
        let (x, y) = (0.3, -0.2);
        let (nx, ny) = c.next(x, y);
        assert_eq!(nx, (-1.4 * y).sin() + 1.0 * (-1.4 * x).cos());
        assert_eq!(ny, (1.6 * x).sin() + 0.7 * (1.6 * y).cos());
    }

    #[test]
    fn henon_settles_on_a_fixed_point_for_small_a() {
        let h = HenonAttractor::from_params(&[0.2, 0.3]);
        assert!(lyapunov_exponent(&h, 10_000) < 0.0);
    }

    #[test]
    fn escaping_trajectory_has_nan_lyapunov_exponent() {
        let h = HenonAttractor::from_params(&[3.0, 0.3]);
        assert!(lyapunov_exponent(&h, 10_000).is_nan());
    }

    #[test]
    fn with_param_changes_only_that_param() {
        let s = SprottQuadraticAttractor::from_params(&[0.1; 12]).with_param(11, -0.5);
        assert_eq!(s.params()[..11], [0.1; 11]);
        assert_eq!(s.params()[11], -0.5);
    }

    #[test]
    fn parse_params_round_trips() {
        let names = CliffordAttractor::PARAM_NAMES;
        let p = [1.378_6, -0.83, 1.004_6, -2.43];
        assert_eq!(
            parse_params(&format_params(names, &p), names),
            Some(p.to_vec())
        );
        assert_eq!(
            parse_params("a=1,b=2,c=3,d=4,iters=10", names),
            Some(vec![1.0, 2.0, 3.0, 4.0])
        );
        let names = SprottQuadraticAttractor::PARAM_NAMES;
        let p: Vec<f64> = (0..12).map(|i| i as f64 / 10.0).collect();
        assert_eq!(parse_params(&format_params(names, &p), names), Some(p));
    }

    #[test]
    fn parse_params_rejects_malformed_lines() {
        let names = CliffordAttractor::PARAM_NAMES;
        for line in ["", "a", "a=1,b=2,c=3", "a=1,b=2,c=3,e=4", "a=1,b=x,c=3,d=4"] {
            assert_eq!(parse_params(line, names), None, "{line:?}");
        }
    }
}
