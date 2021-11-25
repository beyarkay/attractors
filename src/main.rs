
/// Collect common functions used in defining strange attractors.
///
/// Examples of strange attractors:
/// - [Clifford Attractors](http://paulbourke.net/fractals/clifford/)
/// - [De Jong Attractors](http://paulbourke.net/fractals/peterdejong/)
/// - [Lyapunov Exponent Attractors](http://paulbourke.net/fractals/lyapunov/)
/// - [Sprott Polynomial Attractors](http://paulbourke.net/fractals/sprott/)
/// - [Juan Attractors](http://paulbourke.net/fractals/juan2/)
/// - [Den Tsucs Attractors](http://paulbourke.net/fractals/tsucs/)
/// - [Sánchez 'Bad Hairday' Attractors](http://paulbourke.net/fractals/2dmap/), also see
/// [here](https://www.r-bloggers.com/2019/10/strange-attractors-an-r-experiment-about-maths-recursivity-and-creative-coding/)
/// - [Arneodo Attractors](http://paulbourke.net/fractals/arneodo/)
/// - [Catrián Attractors](http://paulbourke.net/fractals/JuanCatrian/)
/// - [Burke-Shaw Attractors](http://paulbourke.net/fractals/burkeshaw/)
/// - [Yu-Wang Attractors](http://paulbourke.net/fractals/yuwang/)
trait Attractor {
    /// Create a new Attractor from the vector of parameters `params`.
    fn new(params: Vec<f64>) -> Self;

    /// Given an xy position, mutate x and y to be the next position based on the Attractor's
    /// formula.
    fn step(&mut self, x: &mut f64, y: &mut f64);

    /// Change the parameters of the Attractor. 
    ///
    /// If an element in params is `None`, then that parameter will remain how it was. If an
    /// element in `params` is `Some<f64>`, then that value will be unpacked into the corresponding
    /// parameter.
    fn set_params(&mut self, params: Vec<Option<f64>>);
}

/// A Clifford Attractor, as discovered by [Clifford A
/// Pickover](https://en.wikipedia.org/wiki/Clifford_A._Pickover)
///
/// The parameters a, b, c, d are all used in the formula for the attractor, which takes in one xy
/// position and returns a new xy position. Repeatedly doing this operation leads to the Clifford
/// attractor. The formula is:
/// ```
/// x_new = sin(a * y) + c * cos(a * x)
/// y_new = sin(b * x) + d * cos(b * y)
/// ```
struct CliffordAttractor {
    /// Parameter a is only used in calculating the new x value.
    a: f64, 
    /// Parameter b is only used in calculating the new y value.
    b: f64, 
    /// Parameter c roughly dictates the min and max x values.
    c: f64, 
    /// Parameter d roughly dictates the min and max y values.
    d: f64,
    /// The minimum x value, calculated as: `min(sin()) - |c| * min(cos()) == -1.0 - c.abs()`
    xmin: f64, 
    /// The maximum x value, calculated as: `max(sin()) + |c| * max(cos()) ==  1.0 + c.abs()`
    xmax: f64,
    /// The minimum y value, calculated as: `min(sin()) - |d| * min(cos()) == -1.0 - d.abs()`
    ymin: f64, 
    /// The maximum y value, calculated as: `max(sin()) + |d| * max(cos()) ==  1.0 + d.abs()`
    ymax: f64
}

impl Attractor for CliffordAttractor { 
    /// Create a new Clifford attractor from the vector of parameters `params`
    fn new(params: Vec<f64>) -> Self {
        assert!(params.len() == 4,
            "Clifford Attractors require 4 parameters (a, b, c, d) but you only gave {}", params.len());

        CliffordAttractor {
            a: params[0],
            b: params[1],
            c: params[2],
            d: params[3],
            xmin: -1.0 - params[2].abs(), // min(sin()) - |c| * min(cos())
            xmax:  1.0 + params[2].abs(), // max(sin()) + |c| * max(cos())
            ymin: -1.0 - params[3].abs(), // min(sin()) - |d| * min(cos())
            ymax:  1.0 + params[3].abs(), // max(sin()) + |d| * max(cos())
        }
    }

    /// Given an xy position, mutate x and y to be the next position based on the Clifford attractor
    /// formula: 
    /// ```
    /// x_new = sin(a * y) + c * cos(a * x)
    /// y_new = sin(b * x) + d * cos(b * y)
    /// ```
    fn step(&mut self, x: &mut f64, y: &mut f64) {
        *x = (self.a * *y).sin() + self.c * (self.a * *x).cos();
        *y = (self.b * *x).sin() + self.d * (self.b * *y).cos();
    }


    /// Set the parameters a, b, c, d of the Clifford attractor and recalculate the x, y min and max
    /// values as needed. 
    ///
    /// A `Vec<Option<f64>>` is required, where the length of the vector must be
    /// 4 If an item in the vector is `Some`, then that item will be set to the value of a, b, c,
    /// or d based on the index of the item
    fn set_params(&mut self, params: Vec<Option<f64>>) {
        assert!(params.len() == 4,
            "Clifford Attractors require 4 parameters (a, b, c, d) but you only gave {}", params.len());
        // Go through each parameter and check if it needs to be updated
        match params[0] {
            Some(a) => self.a = a,
            None => (),
        }
        match params[2] {
            Some(b) => self.b = b,
            None => (),
        }
        match params[2] {
            Some(c) => {
                self.c = c;
                // Recalculate the xmin and xmax values
                self.xmin = -1.0 - c.abs();
                self.xmax =  1.0 + c.abs();
            },
            None => (),
        }
        match params[3] {
            Some(d) => {
                self.d = d;
                // Recalculate the ymin and ymax values
                self.ymin = -1.0 - d.abs();
                self.ymax =  1.0 + d.abs();
            },
            None => (),
        }
    }
}

fn main() {
    println!("Hello, world!");
}
