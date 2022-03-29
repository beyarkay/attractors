#![feature(test)]
extern crate test;
extern crate minifb;
mod attractors;
use std::fmt::Display;

use crate::attractors::*;
use minifb::{Key, Window, WindowOptions};
use palette::{FromColor, Lch, Srgb};
use rand::Rng;

const WIDTH: usize = 900;
const HEIGHT: usize = 900;
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
        Command { // Clifford A
            keys: vec![Key::Key1],
            action: Box::new(|clifford, _buffer, keys, _lch| {
                let sign = if keys.contains(&Key::LeftShift) { -1.0 } else { 1.0 };
                clifford.set_params(vec![Some(clifford.a + 0.01 * sign), None, None, None]);
                clifford.reset();
                clifford.step(100_000);
            }),
            description: "Increase or decrease `clifford.a` by 0.01".to_string(),
            enabled: true,
        },
        Command { // Clifford B
            keys: vec![Key::Key2],
            action: Box::new(|clifford, _buffer, keys, _lch| {
                let sign = if keys.contains(&Key::LeftShift) { -1.0 } else { 1.0 };
                clifford.set_params(vec![None, Some(clifford.b + 0.01 * sign), None, None]);
                clifford.reset();
                clifford.step(100_000);
            }),
            description: "Increase or decrease `clifford.b` by 0.01".to_string(),
            enabled: true,
        },
        Command { // Clifford C
            keys: vec![Key::Key3],
            action: Box::new(|clifford, _buffer, keys, _lch| {
                let sign = if keys.contains(&Key::LeftShift) { -1.0 } else { 1.0 };
                clifford.set_params(vec![None, None, Some(clifford.c + 0.01 * sign), None]);
                clifford.reset();
                clifford.step(100_000);
            }),
            description: "Increase or decrease `clifford.c` by 0.01".to_string(),
            enabled: true,
        },
        Command { // Clifford D
            keys: vec![Key::Key4],
            action: Box::new(|clifford, _buffer, keys, _lch| {
                let sign = if keys.contains(&Key::LeftShift) { -1.0 } else { 1.0 };
                clifford.set_params(vec![None, None, None, Some(clifford.d + 0.01 * sign)]);
                clifford.reset();
                clifford.step(100_000);
            }),
            description: "Increase or decrease `clifford.d` by 0.01".to_string(),
            enabled: true,
        },
        Command { // Light Intercept
            keys: vec![Key::Q],
            action: Box::new(|clifford, _buffer, keys, lch| {
                let sign = if keys.contains(&Key::LeftShift) { -1.0 } else { 1.0 };
                lch.light_intercept += 0.05 * sign;
                println!("{:#}", lch);
                clifford.step(1);
            }),
            description: "Increase or decrease the LCH light intercept by 0.05".to_string(),
            enabled: true,
        },
        Command { // Light Slope
            keys: vec![Key::A],
            action: Box::new(|clifford, _buffer, keys, lch| {
                let sign = if keys.contains(&Key::LeftShift) { -1.0 } else { 1.0 };
                lch.light_slope += 0.05 * sign;
                println!("{:#}", lch);
                clifford.step(1);
            }),
            description: "Increase or decrease the LCH light slope by 0.05".to_string(),
            enabled: true,
        },
        Command { // Chroma Intercept
            keys: vec![Key::W],
            action: Box::new(|clifford, _buffer, keys, lch| {
                let sign = if keys.contains(&Key::LeftShift) { -1.0 } else { 1.0 };
                lch.chroma_intercept += 0.05 * sign;
                println!("{:#}", lch);
                clifford.step(1);
            }),
            description: "Increase or decrease the LCH chroma intercept by 0.05".to_string(),
            enabled: true,
        },
        Command { // Chroma Slope
            keys: vec![Key::S],
            action: Box::new(|clifford, _buffer, keys, lch| {
                let sign = if keys.contains(&Key::LeftShift) { -1.0 } else { 1.0 };
                lch.chroma_slope += 0.05 * sign;
                println!("{:#}", lch);
                clifford.step(1);
            }),
            description: "Increase or decrease the LCH chroma slope by 0.05".to_string(),
            enabled: true,
        },
        Command { // Hue Intercept
            keys: vec![Key::E],
            action: Box::new(|clifford, _buffer, keys, lch| {
                let sign = if keys.contains(&Key::LeftShift) { -1.0 } else { 1.0 };
                lch.hue_intercept += 0.05 * sign;
                println!("{:#}", lch);
                clifford.step(1);
            }),
            description: "Increase or decrease the LCH hue intercept by 0.05".to_string(),
            enabled: true,
        },
        Command { // Hue Slope
            keys: vec![Key::D],
            action: Box::new(|clifford, _buffer, keys, lch| {
                let sign = if keys.contains(&Key::LeftShift) { -1.0 } else { 1.0 };
                lch.hue_slope += 0.05 * sign;
                println!("{:#}", lch);
                clifford.step(1);
            }),
            description: "Increase or decrease the LCH hue slope by 0.05".to_string(),
            enabled: true,
        },
        Command { 
            keys: vec![Key::R],
            action: Box::new(|clifford, buffer, keys, lch| {
                for item in buffer.iter_mut() { *item = 0; }
                let mut rng = rand::thread_rng();
                clifford.set_params(vec![
                                    Some(rng.gen_range(-2.0..2.0)),
                                    Some(rng.gen_range(-2.0..2.0)),
                                    Some(rng.gen_range(-2.0..2.0)),
                                    Some(rng.gen_range(-2.0..2.0)),
                ]);
                clifford.reset();
                clifford.step(50_000);
            }),
            description: "Randomize the Clifford parameters and re-run the attractor with these new parameters".to_string(),
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


    // While the window is open and we want to actually draw things
    // Fewer steps => better responsiveness to keypresses, but 
    // slower generation overall
    print!("Stepping...");
    clifford.step(FIRST_DRAW_SIZE);
    println!("done");
    let mut densities;
    while window.is_open() && !window.is_key_down(Key::Escape) {
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
            if cmd.keys.iter().all(|k| wind_keys.contains(k)) {
                (cmd.action)(&mut clifford, &mut buffer, &wind_keys, &mut lch);
            }
        }
        window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
    }
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

/// Convert CIE Light Chroma Hue to a bit-packed u32 value. https://css.land/lch/
/// l, c, and h are all within [0, 1]
/// Light -> 0.0 is black, 0.5 is full color, 1.0 is  white
/// Chroma -> 0.0 is grey, 1.0 is full colourfulness
/// Hue -> 0.0 to 1.0 is: pink, red, orange, yellow, green, light blue, dark blue, purple, pink
fn lch_to_u32(l: f64, c: f64, h: f64) -> u32 {
    let lch = Lch::new(l * 100.0, c * 132.0, h * 360.0);
    let rgb = Srgb::from_color(lch);
    return argb_to_u32(0, (rgb.red * 255.0) as u8, (rgb.green * 255.0) as u8, (rgb.blue * 255.0) as u8);
}
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
