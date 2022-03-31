#![feature(test)]
extern crate test;
extern crate minifb;
mod attractors;
use std::fmt::Display;

use crate::attractors::*;
use image::{RgbImage, ImageBuffer};
use minifb::{Key, Window, WindowOptions};
use rand::Rng;

const A2_300_DPI_WIDTH: usize = 7016;
const A2_300_DPI_HEIGHT: usize = 4961;
const A3_300_DPI_WIDTH: usize =  4961;
const A3_300_DPI_HEIGHT: usize =  3508;
const A4_300_DPI_WIDTH: usize =  3508;
const A4_300_DPI_HEIGHT: usize =  2480;
const DIAG_WIDTH: usize = 300;
const DIAG_HEIGHT: usize = 100;
const SCREEN_WIDTH: usize = 900;
const SCREEN_HEIGHT: usize = 900;
const WIDTH: usize = SCREEN_WIDTH;
const HEIGHT: usize = SCREEN_HEIGHT;
const FIRST_DRAW_SIZE: usize = 9_000_000;

fn main() {
    let mut _mode: i8 = 1;
    // Create parameters for the clifford attractor
    let mut clifford: CliffordAttractor = CliffordAttractor::new(vec![ -1.4, 1.6, 1.0, 0.7 ]);
    clifford.to_file(format!(
            "cache/clifford/{}-a={}-b={}-c={}-d={}.txt", 
            CliffordAttractor::NAME, clifford.a, clifford.b, clifford.c, clifford.d
            ).to_string());

    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];
    let mut window = Window::new(
        "Strange Attractors (hold esc to exit)",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
    ).unwrap_or_else(|e| { panic!("{}", e); });

    let commands = vec![
        Command { // j -> a--; J -> b--;
            keys: vec![Key::J],
            action: Box::new(|clifford, _buffer, keys, _lch| {
                if keys.contains(&Key::LeftShift) {
                    // b--
                    clifford.set_params(vec![None, Some(clifford.b - 0.01), None, None]);
                } else {
                    // a--
                    clifford.set_params(vec![Some(clifford.a - 0.01), None, None, None]);
                };
                clifford.reset();
                clifford.step(100_000);
            }),
            description: "j -> a--; J -> b--;".to_string(),
            enabled: true,
        },
        Command { // k -> a++; K -> b++;
            keys: vec![Key::K],
            action: Box::new(|clifford, _buffer, keys, _lch| {
                if keys.contains(&Key::LeftShift) {
                    // b++
                    clifford.set_params(vec![None, Some(clifford.b + 0.01), None, None]);
                } else {
                    // a++
                    clifford.set_params(vec![Some(clifford.a + 0.01), None, None, None]);
                };
                clifford.reset();
                clifford.step(100_000);
            }),
            description: "k -> a++; K -> b++;".to_string(),
            enabled: true,
        },
        Command { // h -> c--; H -> d--;
            keys: vec![Key::H],
            action: Box::new(|clifford, _buffer, keys, _lch| {
                if keys.contains(&Key::LeftShift) {
                    // c--
                    clifford.set_params(vec![None, None, Some(clifford.c - 0.01), None]);
                } else {
                    // d--
                    clifford.set_params(vec![None, None, None, Some(clifford.d - 0.01)]);
                };
                clifford.reset();
                clifford.step(100_000);
            }),
            description: "h -> c--; H -> d--;".to_string(),
            enabled: true,
        },
        Command { // l -> c++; L -> d++;
            keys: vec![Key::L],
            action: Box::new(|clifford, _buffer, keys, _lch| {
                if keys.contains(&Key::LeftShift) {
                    // c++
                    clifford.set_params(vec![None, None, Some(clifford.c + 0.01), None]);
                } else {
                    // d++
                    clifford.set_params(vec![None, None, None, Some(clifford.d + 0.01)]);
                };
                clifford.reset();
                clifford.step(100_000);
            }),
            description: "l -> c++; L -> d++;".to_string(),
            enabled: true,
        },
        Command { // Light Intercept
            keys: vec![Key::Q],
            action: Box::new(|clifford, _buffer, keys, lch| {
                let sign = if keys.contains(&Key::LeftShift) { -1.0 } else { 1.0 };
                lch.light_intercept += 0.01 * sign;
                println!("{:#}", lch);
                clifford.step(1);
            }),
            description: "Increase or decrease the LCH light intercept by 0.01".to_string(),
            enabled: false,
        },
        Command { // Light Slope
            keys: vec![Key::A],
            action: Box::new(|clifford, _buffer, keys, lch| {
                let sign = if keys.contains(&Key::LeftShift) { -1.0 } else { 1.0 };
                lch.light_slope += 0.01 * sign;
                println!("{:#}", lch);
                clifford.step(1);
            }),
            description: "Increase or decrease the LCH light slope by 0.01".to_string(),
            enabled: false,
        },
        Command { // Chroma Intercept
            keys: vec![Key::W],
            action: Box::new(|clifford, _buffer, keys, lch| {
                let sign = if keys.contains(&Key::LeftShift) { -1.0 } else { 1.0 };
                lch.chroma_intercept += 0.01 * sign;
                println!("{:#}", lch);
                clifford.step(1);
            }),
            description: "Increase or decrease the LCH chroma intercept by 0.01".to_string(),
            enabled: false,
        },
        Command { // Chroma Slope
            keys: vec![Key::S],
            action: Box::new(|clifford, _buffer, keys, lch| {
                let sign = if keys.contains(&Key::LeftShift) { -1.0 } else { 1.0 };
                lch.chroma_slope += 0.01 * sign;
                println!("{:#}", lch);
                clifford.step(1);
            }),
            description: "Increase or decrease the LCH chroma slope by 0.01".to_string(),
            enabled: false,
        },
        Command { // Hue Intercept
            keys: vec![Key::E],
            action: Box::new(|clifford, _buffer, keys, lch| {
                let sign = if keys.contains(&Key::LeftShift) { -1.0 } else { 1.0 };
                lch.hue_intercept += 0.01 * sign;
                println!("{:#}", lch);
                clifford.step(1);
            }),
            description: "Increase or decrease the LCH hue intercept by 0.01".to_string(),
            enabled: true,
        },
        Command { // Hue Slope
            keys: vec![Key::D],
            action: Box::new(|clifford, _buffer, keys, lch| {
                let sign = if keys.contains(&Key::LeftShift) { -1.0 } else { 1.0 };
                lch.hue_slope += 0.01 * sign;
                println!("{:#}", lch);
                clifford.step(1);
            }),
            description: "Increase or decrease the LCH hue slope by 0.01".to_string(),
            enabled: false,
        },
        Command { // Reset and randomise
            keys: vec![Key::R],
            action: Box::new(|clifford, buffer, keys, _lch| {
                for item in buffer.iter_mut() { *item = 0; }
                if keys.contains(&Key::LeftShift) && clifford.param_history.len() > 0 { 
                    clifford.set_params( clifford.param_history.iter().nth_back(2).expect("param history was empty")
                            .clone()
                            .into_iter()
                            .map(|p| Some(p))
                            .collect()
                    );
                } else { 
                    let mut rng = rand::thread_rng();
                    clifford.set_params(vec![
                                        Some(rng.gen_range(-4.0..4.0)),
                                        Some(rng.gen_range(-4.0..4.0)),
                                        Some(rng.gen_range(-4.0..4.0)),
                                        Some(rng.gen_range(-4.0..4.0)),
                    ]);
                };
                clifford.reset();
                clifford.step(1_000_000);
            }),
            description: "Randomize the Clifford parameters and re-run the attractor with these new parameters".to_string(),
            enabled: true,
        },
        Command { // Print to disc
            keys: vec![Key::P],
            action: Box::new(|clifford, _buffer, _keys, lch| {
                let filename = format!("cache/clifford/a={}_b={}_c={}_d={}_iters={}.png", clifford.a, clifford.b, clifford.c, clifford.d, clifford.history.len());
                println!("Saving data to {}", filename);
                let mut image: RgbImage = ImageBuffer::new(7000, 7000);
                while clifford.history.len() < 10_000_000 {
                    clifford.step(1_000_000);
                }
                let densities = clifford.get_densities(7000, 7000);
                for (i, val) in densities.iter().enumerate() {
                    let packed = hsla_to_u32(
                        val * lch.hue_slope + lch.hue_intercept,
                        val * lch.chroma_slope + lch.chroma_intercept,
                        val.powf(0.3) * lch.light_slope + lch.light_intercept,
                        0.0,
                    );
                    let x = i % 7000;
                    let y = i / 7000;
                    let (_a, r, g, b) = u32_to_argb(packed);
                    image.put_pixel(x as u32, y as u32, image::Rgb([r, g, b]));
                }
                image.save(filename).unwrap();
            }),
            description: "Print the attractor to disc".to_string(),
            enabled: true,
        },
        Command { // Change from black bg to white bg
            keys: vec![Key::I],
            action: Box::new(|clifford, _buffer, _keys, lch| {
                if lch.light_intercept == 1.0 {
                    // Dark background
                    lch.light_intercept = 0.0;
                    lch.light_slope = 1.0;
                    lch.chroma_intercept = 1.5; 
                    lch.chroma_slope = 0.2; 
                } else {
                    // Light background
                    lch.light_intercept = 1.0;
                    lch.light_slope = -1.0;
                    lch.chroma_intercept = 0.7;
                    lch.chroma_slope = 1.5; 
                }
                println!("Inverted colours:\n{:#}", lch);
                clifford.step(1);
            }),
            description: "Increase or decrease the LCH hue slope by 0.01".to_string(),
            enabled: true,
        },
        ];
    println!("=== List of Commands ===");
    for command in commands.iter() {
        println!("`{:?}` => {}", command.keys, command.description);
    }

    // These parameters have been manually tuned
    let mut lch = LchParams {
        light_intercept:  0.0, // no touchie
        light_slope: 1.0,
        chroma_intercept: 1.5,
        chroma_slope: 0.2,
        hue_intercept: 0.45,
        hue_slope: 0.15, // values over 0.5 give a bit of a blowout effect
    };

    let mut diagnostics = Window::new(
        "Diagnostics",
        DIAG_WIDTH,
        DIAG_HEIGHT,
        WindowOptions {
            ..WindowOptions::default()
        },
    ).unwrap();

    clifford.step(FIRST_DRAW_SIZE);
    let mut densities;
    let mut diag_buf = vec![0u32; DIAG_WIDTH * DIAG_HEIGHT];
    while (window.is_open() && !window.is_key_down(Key::Escape)) && 
          (diagnostics.is_open() && !diagnostics.is_key_down(Key::Escape)) {
        // Then use those generated points to draw onto the buffer in 
        // the appropriate spaces
        if clifford.history.len() < 20_000_000 {
            clifford.step(100_000);
        }
        densities = clifford.get_densities(WIDTH, HEIGHT);
        for (i, item) in buffer.iter_mut().enumerate() {
            let val: f64 = densities[i];
            *item = hsla_to_u32(
                val * lch.hue_slope + lch.hue_intercept,
                val * lch.chroma_slope + lch.chroma_intercept,
                val.powf(0.3) * lch.light_slope + lch.light_intercept,
                0.0,
            );
        }
        let wind_keys = window.get_keys();
        for cmd in &commands {
            // check if the currently pressed keys match any of the commands' required keys
            if cmd.enabled && cmd.keys.iter().all(|k| wind_keys.contains(k)) {
                (cmd.action)(&mut clifford, &mut buffer, &wind_keys, &mut lch);
            }
        }
        window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
        update_diagnostics(&mut diag_buf, &clifford);
        diagnostics.update_with_buffer(&diag_buf, DIAG_WIDTH, DIAG_HEIGHT).unwrap();
    }
}

fn update_diagnostics(diag_buf: &mut Vec<u32>, clifford: &CliffordAttractor) {
    for (idx, param) in clifford.param_history.iter().enumerate() {
        let a_in_bounds = -5.0 <= param[0] && param[0] < 5.0;
        let b_in_bounds = -5.0 <= param[1] && param[1] < 5.0;
        let c_in_bounds = -5.0 <= param[2] && param[2] < 5.0;
        let d_in_bounds = -5.0 <= param[3] && param[3] < 5.0;
        if  a_in_bounds && b_in_bounds && c_in_bounds && d_in_bounds {
            let a = (param[0] + 5.0) / 10.0 * 100.0; // horizontal axis
            let b = (param[1] + 5.0) / 10.0 * 100.0; // vertical axis
            let c = ((param[2] + 5.0) / 10.0 * 200.0 + 50.0) as u8; // Red colour axis
            let d = ((param[3] + 5.0) / 10.0 * 200.0 + 50.0) as u8; // Green color axis
            let pos = b as usize * DIAG_WIDTH + a as usize;
            diag_buf[pos] = argb_to_u32(0, c, d, 255);
        }
    }
}

fn u32_to_argb(packed: u32) -> (u8, u8, u8, u8) {
    let a = ((0xFF_00_00_00 & packed) >> 24) as u8;
    let r = ((0x00_FF_00_00 & packed) >> 16) as u8;
    let g = ((0x00_00_FF_00 & packed) >> 8) as u8;
    let b = (0x00_00_00_FF & packed) as u8;
    return (a, r, g, b);
}

fn argb_to_u32(a: u8, r: u8, g: u8, b: u8) -> u32 {
    let (a, r, g, b) = (a as u32, r as u32, g as u32, b as u32);
    (a << 24) | (r << 16) | (g << 8) | b
}

/// Convert a Hue Saturation Light Alpha colour to a bit-packed u32. Alpha is currently ignored.
fn hsla_to_u32(h: f64, s: f64, l: f64, _a: f64) -> u32 {
    // Converted to rust from JS taken from:
    // https://stackoverflow.com/a/9493060
    let r: u8;
    let g: u8;
    let b: u8;
    let a: u8 = 1;

    // If saturation is zero, then the color is just grey => all red, 
    // green, blue components are equal
    if s == 0.0 {
        r = (l * 255.0) as u8;
        g = (l * 255.0) as u8;
        b = (l * 255.0) as u8;
    } else {
        fn hue_to_rgb_floats(p: f64, q: f64, mut t: f64) -> f64 {
            // ensure 0.0 <= t <= 1.0
            if t < 0.0 { t += 1.0 };
            if t > 1.0 { t -= 1.0 };
            // I've got no clue how this works
            if t < 1.0/6.0 { return p + (q - p) * 6.0 * t };
            if t < 1.0/2.0 { return q };
            if t < 2.0/3.0 { return p + (q - p) * (2.0/3.0 - t) * 6.0 };
            return p;
        }

        // I've got no clue how this works
        let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
        // I've got no clue how this works
        let p = 2.0 * l - q;
        r = (255.0 * hue_to_rgb_floats(p, q, h + 1.0/3.0) ) as u8;
        g = (255.0 * hue_to_rgb_floats(p, q, h) ) as u8;
        b = (255.0 * hue_to_rgb_floats(p, q, h - 1.0/3.0) ) as u8;
    }
    return argb_to_u32(a, r, g, b)
}

// /// Convert CIE Light Chroma Hue to a bit-packed u32 value. https://css.land/lch/
// /// l, c, and h are all within [0, 1]
// /// Light -> 0.0 is black, 0.5 is full color, 1.0 is  white
// /// Chroma -> 0.0 is grey, 1.0 is full colourfulness
// /// Hue -> 0.0 to 1.0 is: pink, red, orange, yellow, green, light blue, dark blue, purple, pink
// fn lch_to_u32(l: f64, c: f64, h: f64) -> u32 {
//     let lch = Lch::new(l * 100.0, c * 132.0, h * 360.0);
//     let rgb = Srgb::from_color(lch);
//     return argb_to_u32(0, (rgb.red * 255.0) as u8, (rgb.green * 255.0) as u8, (rgb.blue * 255.0) as u8);
// }
// TODO plot the keyframes as lines tracing the a,b,c,d parameters along the 
// bottom of the screen

/// A common format for commands so that a help file can be printed out easily in the format 
/// `key` -> `description`.
struct Command { 
    /// The key which triggers the `action`.
    keys: Vec<Key>,
    /// A function called when `key` is pressed.
    action: Box<dyn Fn(&mut CliffordAttractor, &mut Vec<u32>, &Vec<Key>, &mut LchParams) -> ()>,
    /// A one-line description of what `action` does.
    description: String,
    /// `action` is only called if `key` is pressed and `enabled` is true.
    enabled: bool,
}


/// Contains the constants that get multiplied by the value at each pixel in order to convert that
/// scalar value to a color in LCH space. The conversion is done as:
/// ``` 
/// light_component  = val * light_slope  + light_intercept
/// chroma_component = val * chroma_slope + chroma_intercept
/// hue_component    = val * hue_slope    + hue_intercept
/// ```
/// For example
struct LchParams {
    light_intercept: f64,
    /// Light -> 0.0 is black, 0.5 is full color, 1.0 is  white
    light_slope: f64,
    chroma_intercept: f64,
    /// Chroma -> 0.0 is grey, 1.0 is full colourfulness
    chroma_slope: f64,
    hue_intercept: f64,
    /// Hue -> 0.0 to 1.0 is: pink, red, orange, yellow, green, light blue, dark blue, purple, pink
    hue_slope: f64,
}

impl Display for LchParams {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "LchParams:\n  light = val * {light_slope:+.4} + {light_intercept:+.4}\n  chroma = val * {chroma_slope:+.4} + {chroma_intercept:+.4}\n  hue = val * {hue_slope:+.4} + {hue_intercept:+.4}",
               light_slope=self.light_slope,
               light_intercept=self.light_intercept,
               chroma_slope=self.chroma_slope,
               chroma_intercept=self.chroma_intercept,
               hue_slope=self.hue_slope,
               hue_intercept=self.hue_intercept,
        )
    }
}
