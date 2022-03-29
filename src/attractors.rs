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
/// [here](https://www.r-bloggers.com/2019/10/strange-attractors-an-r-experiment-about-maths-recursivity-and-creative-coding/)
/// - [Arneodo Attractors](http://paulbourke.net/fractals/arneodo/)
/// - [Catrián Attractors](http://paulbourke.net/fractals/JuanCatrian/)
/// - [Burke-Shaw Attractors](http://paulbourke.net/fractals/burkeshaw/)
/// - [Yu-Wang Attractors](http://paulbourke.net/fractals/yuwang/)
use std::{io::Write, fmt::{Display, self}};
pub trait Attractor {
    /// A name for the attractor matching [a-zA-Z]+, used when saving a sequence of points to file
    const NAME: &'static str;
    /// The number of dimensions the attractor lives in. Almost always 2 or 3.
    const DIMENSIONALITY: u8;
    /// The number of parameters required by the attractor.
    const NUM_PARAMETERS: u8;

    /// Create a new Attractor from the vector of parameters `params`.
    fn new(params: Vec<f64>) -> Self;

    /// Given an xy position, mutate x and y to be the next position based on the Attractor's
    /// formula. Do this for `num_steps` steps, saving each step to `history`.
    fn step(&mut self, num_steps: usize);

    /// Change the parameters of the Attractor.
    ///
    /// If an element in params is `None`, then that parameter will remain how it was. If an
    /// element in `params` is `Some<f64>`, then that value will be unpacked into the corresponding
    /// parameter.
    fn set_params(&mut self, params: Vec<Option<f64>>);

    fn get_densities(&mut self, width: usize, height: usize) -> Vec<f64>;

    fn reset(&mut self);

    /// Save the attractor to a file.
    ///
    /// The attractor's name, it's parameters, and every coordinate taken so far is saved to the
    /// directory named `directory` as a filename determined by the parameters of the attractor to
    /// allow for computer-based retrieval.
    ///
    /// The format of the file is in plain text like:
    /// ```
    /// #<NAME>,<NUM_PARAMETERS>,<NUM_DIMENSIONS>
    /// #<NAME_OF_PARAMETER_1>=<VALUE_OF_PARAMETER_1>
    /// ...
    /// #<NAME_OF_PARAMETER_n>=<VALUE_OF_PARAMETER_n>
    /// 0:<X_POSITION>,<Y_POSITION>,<Z_POSITION>
    /// 1:<X_POSITION>,<Y_POSITION>,<Z_POSITION>
    /// 2:<X_POSITION>,<Y_POSITION>,<Z_POSITION>
    /// 3:<X_POSITION>,<Y_POSITION>,<Z_POSITION>
    /// ...
    /// ```
    /// Where the z-position is only included for 3-dimensional attractors. These files are
    /// designed to be human-readable for debug purposes.
    fn to_file(&mut self, directory: String);
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
#[derive(Debug)]
pub struct CliffordAttractor {
    /// Parameter a is only used in calculating the new x value.
    pub a: f64,
    /// Parameter b is only used in calculating the new y value.
    pub b: f64,
    /// Parameter c roughly dictates the min and max x values.
    pub c: f64,
    /// Parameter d roughly dictates the min and max y values.
    pub d: f64,
    /// The current x value.
    pub x: f64,
    /// The current y value.
    pub y: f64,
    /// The minimum x value, calculated as: `min(sin()) - |c| * min(cos()) == -1.0 - c.abs()`
    xmin: f64,
    /// The maximum x value, calculated as: `max(sin()) + |c| * max(cos()) ==  1.0 + c.abs()`
    xmax: f64,
    /// The minimum y value, calculated as: `min(sin()) - |d| * min(cos()) == -1.0 - d.abs()`
    ymin: f64,
    /// The maximum y value, calculated as: `max(sin()) + |d| * max(cos()) ==  1.0 + d.abs()`
    ymax: f64,
    /// Store all the previously visited points in the history vector
    pub history: Vec<Vec<f64>>,
}

impl Attractor for CliffordAttractor {
    /// The name used to specify the attractor in text files.
    const NAME: &'static str= "clifford";
    /// Clifford attractors live in 2 dimensions.
    const DIMENSIONALITY: u8 = 2;
    /// Clifford attractors require 4 parameters.
    const NUM_PARAMETERS: u8 = 4;

    fn new(params: Vec<f64>) -> Self {
        assert!(params.len() == 4,
        "Clifford Attractors require 4 parameters (a, b, c, d) but you only gave {}", params.len());

        CliffordAttractor {
            // Create a new Clifford attractor from the vector of parameters `params`
            a: params[0],
            b: params[1],
            c: params[2],
            d: params[3],
            x: 0.0,
            y: 0.0,
            xmin: -1.0 - params[2].abs(), // min(sin()) - |c| * min(cos())
            xmax:  1.0 + params[2].abs(), // max(sin()) + |c| * max(cos())
            ymin: -1.0 - params[3].abs(), // min(sin()) - |d| * min(cos())
            ymax:  1.0 + params[3].abs(), // max(sin()) + |d| * max(cos())
            history: vec![vec![0.0, 0.0]]
        }
    }

    /// Given an xy position, mutate x and y to be the next position based on the Clifford attractor
    /// formula:
    /// ```
    /// x_new = sin(a * y) + c * cos(a * x)
    /// y_new = sin(b * x) + d * cos(b * y)
    /// ```
    /// The `num_steps` variable determines how many times this recurrent equation is evaluated. The x
    /// and y values for each individual iteration can be retrieved from the `.history` vector
    /// variable.
    fn step(&mut self, num_steps: usize) {
        let mut xx = self.x.clone();
        let mut yy = self.y.clone();
        for _ in 1..num_steps {
            xx = (self.a * yy).sin() + self.c * (self.a * xx).cos();
            yy = (self.b * xx).sin() + self.d * (self.b * yy).cos();
            self.history.push(vec![ xx, yy ]);
        }
        self.x = xx;
        self.y = yy;
    }

    fn reset(&mut self) {
        self.history =  vec![vec![0.0, 0.0]];
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
        if let Some(a) = params[0] { self.a = a; }
        if let Some(b) = params[1] { self.b = b; }
        if let Some(c) = params[2] {
            self.c = c;
            // Recalculate the xmin and xmax values
            self.xmin = -1.0 - c.abs();
            self.xmax =  1.0 + c.abs();
        }
        if let Some(d) = params[3] {
            self.d = d;
            // Recalculate the ymin and ymax values
            self.ymin = -1.0 - d.abs();
            self.ymax =  1.0 + d.abs();
        }
        println!("{:#}", self);
        // dbg!(self.a, self.b, self.c, self.d);
    }

    /// Given a certain `width` and `height` in pixels, return the density of the 
    /// pixel at `index`, where `index` is equal to:
    /// ```
    ///     index = x_position + width * y_position
    /// ```
    ///
    /// The returned density has been normalised to be in the range [0.0, 1.0]
    /// where 1.0 indicates that pixel was the most frequently landed on, and 
    /// 0.0 indicates that pixel was never landed on.
    // TODO: add padding argument to display the attractor on only part of the 
    // buffer
    fn get_densities(&mut self, width: usize, height: usize) -> Vec<f64> {
        let xrange = self.xmax - self.xmin;
        let yrange = self.ymax - self.ymin;
        let mut densities = vec![0.0; width * height];
        let mut densities_max = 0.0;

        // First loop over the history, flooring all the values to find
        // a histogram of how many times the attractor hit each xy point
        for pos in self.history.iter() {
            let x = (width as f64 * (pos[0] - self.xmin) / xrange).floor() as usize;
            let y = (height as f64 * (pos[1] - self.ymin) / yrange).floor() as usize;
            let i = x + y * width;
            densities[i] += 1.0;
            if densities[i] > densities_max {
                densities_max = densities[i];
            }
        }
        // Then loop over it again, to divide each of the items in the densities 
        // by the maximum
        for idx in &mut densities{
            *idx /= densities_max;
        }
        return densities;
    }

    /// Write the Clifford Attractor to the file named `filename`
    fn to_file(&mut self, filename: String) {
        let mut file = std::fs::File::create(filename).expect("Failed to create file.");
        // The preamble contains various things defining the attractor in question, and every line
        // in the preamble starts with a `#`
        let preamble: String = format!("#{},{},{}\n#a={}\n#b={}\n#c={}\n#d={}\n",
                                        CliffordAttractor::NAME,
                                        CliffordAttractor::NUM_PARAMETERS,
                                        CliffordAttractor::DIMENSIONALITY,
                                        self.a, self.b, self.c, self.d);
        file.write_all(preamble.as_bytes()).expect("Failed to preamble write to file.");
        for (i, item) in self.history.iter().enumerate() {
            let position: String = format!("{}:{},{}\n", i, item[0], item[1]);
            file.write_all(position.as_bytes()).expect("Failed to write position to file.");
        }
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
/// The parameters a, b, c, d are all used in the formula for the attractor, which takes in one xy
/// position and returns a new xy position. Repeatedly doing this operation leads to the De Jong
/// attractor. The formula is:
/// ```
/// x_new = sin(a * y_old) - cos(b * x_old)
/// y_new = sin(c * x_old) - cos(d * y_old)
/// ```
/// Which is similar to the Clifford Attractor, but without scaling constants on the second term.
pub struct DeJongAttractor {
    /// Parameter a
    pub a: f64,
    /// Parameter b
    pub b: f64,
    /// Parameter c
    pub c: f64,
    /// Parameter d
    pub d: f64,
    /// The current x value.
    pub x: f64,
    /// The current y value.
    pub y: f64,
    /// The minimum x value, which is always -2 for DeJong attractors
    xmin: f64,
    /// The maximum x value, which is always 2 for DeJong attractors
    xmax: f64,
    /// The minimum y value, which is always -2 for DeJong attractors
    ymin: f64,
    /// The maximum y value, which is always 2 for DeJong attractors
    ymax: f64,
    /// Store all the previously visited points in the history vector
    pub history: Vec<Vec<f64>>,
}

impl Attractor for DeJongAttractor {
    /// The name used to specify the attractor in text files.
    const NAME: &'static str= "dejong";
    /// DeJong attractors live in 2 dimensions.
    const DIMENSIONALITY: u8 = 2;
    /// DeJong attractors require 4 parameters.
    const NUM_PARAMETERS: u8 = 4;

    fn new(params: Vec<f64>) -> Self {
        assert!(params.len() == 4,
        "DeJong Attractors require 4 parameters (a, b, c, d) but you only gave {}", params.len());

        DeJongAttractor {
            // Create a new DeJong attractor from the vector of parameters `params`
            a: params[0],
            b: params[1],
            c: params[2],
            d: params[3],
            x: 0.0,
            y: 0.0,
            xmin: -2.0, // min(sin()) - min(cos())
            xmax:  2.0, // max(sin()) + max(cos())
            ymin: -2.0, // min(sin()) - min(cos())
            ymax:  2.0, // max(sin()) + max(cos())
            history: vec![vec![0.0, 0.0]]
        }
    }

    /// Given an xy position, mutate x and y to be the next position based on the DeJong attractor
    /// formula:
    /// ```
    /// x_new = sin(a * y_old) - cos(b * x_old)
    /// y_new = sin(c * x_old) - cos(d * y_old)
    /// ```
    /// The `num_steps` variable determines how many times this recurrent equation is evaluated. The x
    /// and y values for each individual iteration can be retrieved from the `.history` vector
    /// variable.
    fn step(&mut self, num_steps: usize) {
        for _ in 1..num_steps {
            self.x = (self.a * self.y).sin() - (self.b * self.x).cos();
            self.y = (self.c * self.x).sin() - (self.d * self.y).cos();
            self.history.push(vec![ self.x, self.y ]);
        }
    }

    fn reset(&mut self) {
        self.history =  vec![vec![0.0, 0.0]];
    }

    /// Set the parameters a, b, c, d of the DeJong attractor and recalculate the x, y min and max
    /// values as needed.
    ///
    /// A `Vec<Option<f64>>` is required, where the length of the vector must be
    /// 4 If an item in the vector is `Some`, then that item will be set to the value of a, b, c,
    /// or d based on the index of the item
    fn set_params(&mut self, params: Vec<Option<f64>>) {
        assert!(params.len() == 4,
        "DeJong Attractors require 4 parameters (a, b, c, d) but you only gave {}", params.len());
        // Go through each parameter and check if it needs to be updated
        if let Some(a) = params[0] { self.a = a; }
        if let Some(b) = params[1] { self.b = b; }
        if let Some(c) = params[2] { self.c = c; }
        if let Some(d) = params[3] { self.d = d; }
    }

    fn get_densities(&mut self, width: usize, height: usize) -> Vec<f64> {
        let xrange = self.xmax - self.xmin;
        let yrange = self.ymax - self.ymin;
        let mut densities = vec![0.0; width * height];
        let mut densities_max = 0.0;

        // First loop over the history, flooring all the values to find
        // a histogram of how many times the attractor hit each xy point
        for pos in &mut self.history {
            let x = ((pos[0] - self.xmin) / xrange).floor() as usize;
            let y = ((pos[1] - self.ymin) / yrange).floor() as usize;
            let i = x + y * width;
            densities[i] += 1.0;
            if densities[i] > densities_max {
                densities_max = densities[i];
            }
        }
        // Then loop over it again, to divide each of the items in the densities 
        // by the maximum
        for idx in &mut densities{
            *idx /= densities_max;
        }
        return densities;
    }

    /// Write the DeJong Attractor to the file named `filename`
    fn to_file(&mut self, filename: String) {
        let mut file = std::fs::File::create(filename).expect("Failed to create file.");
        // The preamble contains various things defining the attractor in question, and every line
        // in the preamble starts with a `#`
        let preamble: String = format!("#{},{},{}\n#a={}\n#b={}\n#c={}\n#d={}\n",
                                        DeJongAttractor::NAME,
                                        DeJongAttractor::NUM_PARAMETERS,
                                        DeJongAttractor::DIMENSIONALITY,
                                        self.a, self.b, self.c, self.d);
        file.write_all(preamble.as_bytes()).expect("Failed to preamble write to file.");
        for (i, item) in self.history.iter().enumerate() {
            let position: String = format!("{}:{},{}\n", i, item[0], item[1]);
            file.write_all(position.as_bytes()).expect("Failed to write position to file.");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use rand::Rng;
    use test::Bencher;
    use super::*;

    #[bench]
    fn bench_clifford_write_to_file_10k(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut clifford: CliffordAttractor = CliffordAttractor::new(vec![
            rng.gen_range(-2.0..2.0),
            rng.gen_range(-2.0..2.0),
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
        ]);

        let fname = format!(
            "cache/clifford/{}-a={}-b={}-c={}-d={}.tmp", 
            CliffordAttractor::NAME, clifford.a, clifford.b, clifford.c, clifford.d
        );

        clifford.step(10_000);
        b.iter(|| clifford.to_file(fname.to_string()));

        // Remove the temporary files once complete
        fs::remove_file(fname).expect("Failed to delete file");
    }

    #[bench]
    fn bench_clifford_iterate_10k(b: &mut Bencher) {
        let mut rng = rand::thread_rng();
        let mut clifford: CliffordAttractor = CliffordAttractor::new(vec![
            rng.gen_range(-2.0..2.0),
            rng.gen_range(-2.0..2.0),
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
        ]);
        b.iter(|| clifford.step(10_000) );
    }
}
