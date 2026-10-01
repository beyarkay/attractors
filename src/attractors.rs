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

    /// One application of the attractor's formula.
    fn next(&self, x: f64, y: f64) -> (f64, f64);
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
}

impl Attractor for CliffordAttractor {
    const NAME: &'static str = "clifford";
    const DEFAULT_PARAMS: [f64; 4] = [-1.4, 1.6, 1.0, 0.7];

    fn new(params: [f64; 4]) -> Self {
        let [a, b, c, d] = params;
        CliffordAttractor { a, b, c, d }
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
}

impl Attractor for DeJongAttractor {
    const NAME: &'static str = "dejong";
    const DEFAULT_PARAMS: [f64; 4] = [-2.0, -2.0, -1.2, 2.0];

    fn new(params: [f64; 4]) -> Self {
        let [a, b, c, d] = params;
        DeJongAttractor { a, b, c, d }
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
    fn clifford_y_uses_previous_x() {
        let c = CliffordAttractor::new(PARAMS);
        let (x, y) = (0.3, -0.2);
        let (nx, ny) = c.next(x, y);
        assert_eq!(nx, (-1.4 * y).sin() + 1.0 * (-1.4 * x).cos());
        assert_eq!(ny, (1.6 * x).sin() + 0.7 * (1.6 * y).cos());
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
