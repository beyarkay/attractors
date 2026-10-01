//! Strange attractors. To add one: write a struct holding its parameters, implement
//! `Attractor` for it, and add it to `all_attractors!` in `lib.rs`.
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

/// A point in the attractor's space. Maps on the plane leave z at 0.
pub type Point = [f64; 3];

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
    const START: Point = [0.0; 3];
    /// Which coordinates of a point are drawn as the image's x and y
    const PROJECTION: (usize, usize) = (0, 1);
    /// How much each key press changes a parameter, to begin with
    const STEP: f64 = 0.1;
    /// Uncounted steps per walker used to estimate bounds when `bounds` returns `None`
    const BOUNDS_SAMPLE_STEPS: usize = 2_000;

    /// `params` has one value per entry of `PARAM_NAMES`
    fn from_params(params: &[f64]) -> Self;

    fn params(&self) -> &[f64];

    /// A box guaranteed to contain every drawn (projected) point of the attractor, if one is
    /// known. Otherwise the sampler estimates one from its warm-up points.
    fn bounds(&self) -> Option<Bounds> {
        None
    }

    /// One step: one application of a map's formula, or one integration step of a flow.
    fn next(&self, p: Point) -> Point;

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
    let mut p = A::START;
    for _ in 0..1_000 {
        p = attractor.next(p);
    }
    let mut q = p;
    q[0] += SEPARATION;
    let mut sum = 0.0;
    for _ in 0..steps {
        p = attractor.next(p);
        q = attractor.next(q);
        if !p.iter().all(|c| c.abs() < ESCAPED) {
            return f64::NAN;
        }
        let d = [0, 1, 2].map(|i| q[i] - p[i]);
        let dist = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        if dist == 0.0 {
            return f64::NEG_INFINITY;
        }
        sum += (dist / SEPARATION).ln();
        q = [0, 1, 2].map(|i| p[i] + d[i] * SEPARATION / dist);
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
    fn next(&self, [x, y, _]: Point) -> Point {
        let [a, b, c, d] = self.p;
        [
            (a * y).sin() + c * (a * x).cos(),
            (b * x).sin() + d * (b * y).cos(),
            0.0,
        ]
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
    fn next(&self, [x, y, _]: Point) -> Point {
        let [a, b, c, d] = self.p;
        [
            (a * y).sin() - (b * x).cos(),
            (c * x).sin() - (d * y).cos(),
            0.0,
        ]
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
    const START: Point = [0.1, 0.1, 0.0];

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
    fn next(&self, [x, y, _]: Point) -> Point {
        let [a, b, c, d] = self.p;
        [
            (b * y).sin() + c * (b * x).sin(),
            (a * x).sin() + d * (a * y).sin(),
            0.0,
        ]
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
    fn next(&self, [x, y, _]: Point) -> Point {
        let [a, b] = self.p;
        [1.0 - a * x * x + y, b * x, 0.0]
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
    const START: Point = [-0.72, -0.64, 0.0];
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
    fn next(&self, [x, y, _]: Point) -> Point {
        let [a, b, c, d] = self.p;
        [
            x * x - y * y + a * x + b * y,
            2.0 * x * y + c * x + d * y,
            0.0,
        ]
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
    const START: Point = [0.05, 0.05, 0.0];

    fn from_params(params: &[f64]) -> Self {
        SprottQuadraticAttractor {
            p: params.try_into().expect("12 parameters"),
        }
    }

    fn params(&self) -> &[f64] {
        &self.p
    }

    #[inline(always)]
    fn next(&self, [x, y, _]: Point) -> Point {
        let [a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12] = self.p;
        [
            a1 + a2 * x + a3 * x * x + a4 * x * y + a5 * y + a6 * y * y,
            a7 + a8 * x + a9 * x * x + a10 * x * y + a11 * y + a12 * y * y,
            0.0,
        ]
    }
}

/// Barry Martin's [Hopalong](https://www.fractalsciencekit.com/types/orbital.htm) orbit
#[derive(Debug, Clone)]
pub struct HopalongAttractor {
    p: [f64; 3],
}

impl Attractor for HopalongAttractor {
    const NAME: &'static str = "hopalong";
    const FORMULA: &'static str = "x' = y - sign(x) sqrt(|b x - c|)\ny' = a - x";
    const PARAM_NAMES: &'static [&'static str] = &["a", "b", "c"];
    const DEFAULT_PARAMS: &'static [f64] = &[1.1, 0.5, 1.0];
    const PARAM_RANGE: (f64, f64) = (-3.0, 3.0);
    /// The orbit never stops spreading outwards (it reaches ~5 after 2k steps, ~19 after 20k,
    /// ~190 after 20M), so frame the region each walker covers in its first 20k steps
    const BOUNDS_SAMPLE_STEPS: usize = 20_000;

    fn from_params(params: &[f64]) -> Self {
        HopalongAttractor {
            p: params.try_into().expect("3 parameters"),
        }
    }

    fn params(&self) -> &[f64] {
        &self.p
    }

    #[inline(always)]
    fn next(&self, [x, y, _]: Point) -> Point {
        let [a, b, c] = self.p;
        [y - x.signum() * (b * x - c).abs().sqrt(), a - x, 0.0]
    }
}

/// The [Gumowski–Mira](https://en.wikipedia.org/wiki/Gumowski%E2%80%93Mira_attractor) map
#[derive(Debug, Clone)]
pub struct GumowskiMiraAttractor {
    p: [f64; 3],
}

impl GumowskiMiraAttractor {
    #[inline(always)]
    fn f(mu: f64, x: f64) -> f64 {
        mu * x + 2.0 * (1.0 - mu) * x * x / (1.0 + x * x)
    }
}

impl Attractor for GumowskiMiraAttractor {
    const NAME: &'static str = "gumowskimira";
    const FORMULA: &'static str = "f(x) = mu x + 2 (1 - mu) x^2 / (1 + x^2)\n\
                                   x' = y + a (1 - s y^2) y + f(x)\n\
                                   y' = -x + f(x')";
    const PARAM_NAMES: &'static [&'static str] = &["a", "s", "mu"];
    const DEFAULT_PARAMS: &'static [f64] = &[0.008, 0.05, -0.496];
    const PARAM_RANGE: (f64, f64) = (-1.0, 1.0);
    const START: Point = [0.1, 0.1, 0.0];

    fn from_params(params: &[f64]) -> Self {
        GumowskiMiraAttractor {
            p: params.try_into().expect("3 parameters"),
        }
    }

    fn params(&self) -> &[f64] {
        &self.p
    }

    #[inline(always)]
    fn next(&self, [x, y, _]: Point) -> Point {
        let [a, s, mu] = self.p;
        let nx = y + a * (1.0 - s * y * y) * y + Self::f(mu, x);
        [nx, -x + Self::f(mu, nx), 0.0]
    }
}

/// Kensuke Ikeda's [map](https://en.wikipedia.org/wiki/Ikeda_map) of light in an optical ring
#[derive(Debug, Clone)]
pub struct IkedaAttractor {
    p: [f64; 1],
}

impl Attractor for IkedaAttractor {
    const NAME: &'static str = "ikeda";
    const FORMULA: &'static str = "t = 0.4 - 6 / (1 + x^2 + y^2)\n\
                                   x' = 1 + u (x cos t - y sin t)\n\
                                   y' = u (x sin t + y cos t)";
    const PARAM_NAMES: &'static [&'static str] = &["u"];
    const DEFAULT_PARAMS: &'static [f64] = &[0.9];
    const PARAM_RANGE: (f64, f64) = (0.0, 1.0);

    fn from_params(params: &[f64]) -> Self {
        IkedaAttractor {
            p: params.try_into().expect("1 parameter"),
        }
    }

    fn params(&self) -> &[f64] {
        &self.p
    }

    #[inline(always)]
    fn next(&self, [x, y, _]: Point) -> Point {
        let [u] = self.p;
        let (sin, cos) = (0.4 - 6.0 / (1.0 + x * x + y * y)).sin_cos();
        [1.0 + u * (x * cos - y * sin), u * (x * sin + y * cos), 0.0]
    }
}

/// The [Duffing map](https://en.wikipedia.org/wiki/Duffing_map), a discrete Duffing oscillator
#[derive(Debug, Clone)]
pub struct DuffingAttractor {
    p: [f64; 2],
}

impl Attractor for DuffingAttractor {
    const NAME: &'static str = "duffing";
    const FORMULA: &'static str = "x' = y\ny' = -b x + a y - y^3";
    const PARAM_NAMES: &'static [&'static str] = &["a", "b"];
    const DEFAULT_PARAMS: &'static [f64] = &[2.75, 0.2];
    const PARAM_RANGE: (f64, f64) = (-3.0, 3.0);
    const START: Point = [0.1, 0.1, 0.0];

    fn from_params(params: &[f64]) -> Self {
        DuffingAttractor {
            p: params.try_into().expect("2 parameters"),
        }
    }

    fn params(&self) -> &[f64] {
        &self.p
    }

    #[inline(always)]
    fn next(&self, [x, y, _]: Point) -> Point {
        let [a, b] = self.p;
        [y, -b * x + a * y - y * y * y, 0.0]
    }
}

/// René Lozi's piecewise-linear cousin of the Hénon map
#[derive(Debug, Clone)]
pub struct LoziAttractor {
    p: [f64; 2],
}

impl Attractor for LoziAttractor {
    const NAME: &'static str = "lozi";
    const FORMULA: &'static str = "x' = 1 - a |x| + y\ny' = b x";
    const PARAM_NAMES: &'static [&'static str] = &["a", "b"];
    const DEFAULT_PARAMS: &'static [f64] = &[1.7, 0.5];
    const PARAM_RANGE: (f64, f64) = (-2.0, 2.0);

    fn from_params(params: &[f64]) -> Self {
        LoziAttractor {
            p: params.try_into().expect("2 parameters"),
        }
    }

    fn params(&self) -> &[f64] {
        &self.p
    }

    #[inline(always)]
    fn next(&self, [x, y, _]: Point) -> Point {
        let [a, b] = self.p;
        [1.0 - a * x.abs() + y, b * x, 0.0]
    }
}

/// Antonio Sánchez's ["Bedhead"](https://www.r-bloggers.com/2019/10/strange-attractors-an-r-experiment-about-maths-recursivity-and-creative-coding/)
#[derive(Debug, Clone)]
pub struct BedheadAttractor {
    p: [f64; 2],
}

impl Attractor for BedheadAttractor {
    const NAME: &'static str = "bedhead";
    const FORMULA: &'static str = "x' = sin(x y / b) y + cos(a x - y)\ny' = x + sin(y) / b";
    const PARAM_NAMES: &'static [&'static str] = &["a", "b"];
    const DEFAULT_PARAMS: &'static [f64] = &[-0.81, -0.92];
    const PARAM_RANGE: (f64, f64) = (-1.0, 1.0);
    const START: Point = [1.0, 1.0, 0.0];

    fn from_params(params: &[f64]) -> Self {
        BedheadAttractor {
            p: params.try_into().expect("2 parameters"),
        }
    }

    fn params(&self) -> &[f64] {
        &self.p
    }

    #[inline(always)]
    fn next(&self, [x, y, _]: Point) -> Point {
        let [a, b] = self.p;
        [
            (x * y / b).sin() * y + (a * x - y).cos(),
            x + y.sin() / b,
            0.0,
        ]
    }
}

/// Johnny Svensson's attractor
#[derive(Debug, Clone)]
pub struct SvenssonAttractor {
    p: [f64; 4],
}

impl Attractor for SvenssonAttractor {
    const NAME: &'static str = "svensson";
    const FORMULA: &'static str = "x' = d sin(a x) - sin(b y)\ny' = c cos(a x) + cos(b y)";
    const PARAM_NAMES: &'static [&'static str] = &["a", "b", "c", "d"];
    const DEFAULT_PARAMS: &'static [f64] = &[1.5, -1.8, 1.6, 0.9];
    const PARAM_RANGE: (f64, f64) = (-3.0, 3.0);

    fn from_params(params: &[f64]) -> Self {
        SvenssonAttractor {
            p: params.try_into().expect("4 parameters"),
        }
    }

    fn params(&self) -> &[f64] {
        &self.p
    }

    fn bounds(&self) -> Option<Bounds> {
        let [_, _, c, d] = self.p;
        Some(Bounds {
            xmin: -1.0 - d.abs(),
            xmax: 1.0 + d.abs(),
            ymin: -1.0 - c.abs(),
            ymax: 1.0 + c.abs(),
        })
    }

    #[inline(always)]
    fn next(&self, [x, y, _]: Point) -> Point {
        let [a, b, c, d] = self.p;
        [
            d * (a * x).sin() - (b * y).sin(),
            c * (a * x).cos() + (b * y).cos(),
            0.0,
        ]
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

    macro_rules! check_all {
        ($($t:ty),*) => {
            $(check_defaults::<$t>();)*
        };
    }

    #[test]
    fn default_params_are_well_formed_and_chaotic() {
        crate::all_attractors!(check_all);
    }

    #[test]
    fn clifford_y_uses_previous_x() {
        let c = CliffordAttractor::from_params(&[-1.4, 1.6, 1.0, 0.7]);
        let (x, y) = (0.3, -0.2);
        let [nx, ny, _] = c.next([x, y, 0.0]);
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
