// minifb is used for rendering the images
extern crate minifb;
use minifb::{Key, Window, WindowOptions};

// attractors.rs contains definitions of various strange attractors
mod attractors;
use crate::attractors::*;

// Everything good starts with something random
use rand::Rng;

const WIDTH: usize = 900;
const HEIGHT: usize = 900;

fn main() {
    // Write a CliffordAttractor to file
    let mut rng = rand::thread_rng();
    let params = vec![
                  rng.gen_range(-3.0..3.0),
                  rng.gen_range(-3.0..3.0),
                  3.0, 
                  3.0
    ];
    let mut clifford: CliffordAttractor = CliffordAttractor::new(params);
    //clifford.to_file(format!(
    //        "cache/clifford/{}-a={}-b={}-c={}-d={}.txt", 
    //        CliffordAttractor::NAME, clifford.a, clifford.b, clifford.c, clifford.d
    //).to_string());

    // Actually draw a Clifford attractor
    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];
    //let mut hud: Vec<u32> = vec![0; WIDTH * HEIGHT];
    //let mut show_hud: bool = false;
    let mut window = Window::new(
        "Attractors (press q to quit)",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
        ).unwrap_or_else(|e| {
        panic!("{}", e);
    });

    // Limit to max ~60 fps update rate
    window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

    // The attractor is recurrent, so set (0,0) to be the starting point
    let mut x = 0.0;
    let mut y = 0.0;
    // While the window is open and we want to actually draw things
    while window.is_open() && !window.is_key_down(Key::Q) {
        // Fewer steps => better responsiveness to keypresses, but 
        // slower generation overall
        print!("Stepping...");
        clifford.step(&mut x, &mut y, 500_000);
        println!("done");
        // Then use those generated points to draw onto the buffer in 
        // the appropriate spaces
        let densities = clifford.get_densities(WIDTH, HEIGHT);
        for (i, item) in buffer.iter_mut().enumerate() {
            let val: f64 = densities[i];
            //if show_hud {
            //    let (a, r, g, b) = u32_to_argb(hud[i]);
            //    let a: f64 = a as f64;
            //    let r: f64 = r as f64;
            //    let g: f64 = g as f64;
            //    let b: f64 = b as f64;
            //    *item = argb_to_u32(
            //        255,
            //        (val * 255.0 * (1.0 - a) + r * a) as u8,
            //        (val * 255.0 * (1.0 - a) + g * a) as u8,
            //        (val * 255.0 * (1.0 - a) + b * a) as u8);
            //} else {
            *item = hsla_to_u32(
                // Hue: 
                //   0 -> 10: red
                //  10 -> 45: orange
                //  45 -> 65: yellow
                //  65 -> 90: lime
                //  90 -> 140: green
                // 140 -> 165: green-blue
                // 165 -> 190: light blue
                // 190 -> 250: dark blue
                // 250 -> 280: purple
                // 280 -> 345: pink
                // 345 -> 359: red
                (140.0 + 50.0 * val) / 255.0,
                // Saturation: 0 is grey/no colour, 0.7 is pastel, 
                // 1 is full colour
                0.8 + 0.2 * val,  // Sat, [0, 1]
                // Light: 0 is black, 0.5 is full colour, 1 is white
                0.0 + 0.6 * val.powf(1.0/2.5),  
                0.9
                );
        }
        window.get_keys().iter().for_each(|key|
            match key {
                Key::A => {
                    //param_pos.a += 0.1;
                    //reset(&mut buffer, &mut attr_count, &mut max_points);
                },
                Key::B => {
                    //param_pos.b += 0.1;
                    //reset(&mut buffer, &mut attr_count, &mut max_points);
                },
                Key::C => {
                    //param_pos.c += 0.1;
                    //reset(&mut buffer, &mut attr_count, &mut max_points);
                },
                Key::D => {
                    //param_pos.d += 0.1;
                    //reset(&mut buffer, &mut attr_count, &mut max_points);
                },
                Key::H => {
                    //show_hud = !show_hud;
                },
                Key::S => {
                    //param_pos.a = -1.032;
                    //param_pos.b = -1.731;
                    //param_pos.c = -0.286;
                    //param_pos.d = 2.92;
                },
                Key::R => {
                    rng = rand::thread_rng();
                    clifford.set_params(vec![
                               Some(rng.gen_range(-5.0..5.0)),
                               Some(rng.gen_range(-5.0..5.0)),
                               None,
                               None,
                    ]);
                    //clifford.reset();
                },
                _ => (),
            }
        );
        // We unwrap here as we want this code to exit if it fails. Real applications may want to handle this in a different way
        window.update_with_buffer(&buffer, WIDTH, HEIGHT)
            .unwrap();
        }
    }


//fn reset(buffer: &mut Vec<u32>, attr_count: &mut Vec<u32>, max_points: &mut u32) {
//    *max_points = 0;
//    *attr_count = vec![0; attr_count.len()];
//    *buffer = vec![0; buffer.len()];
//}

//fn count_to_brightness(count: u32, max_points: u32, bias_towards_one: f64) -> f64 {
//    // short-circuit for common situations, to avoid floating point division
//    if count == 0 {
//        return 0.0;
//    } else if count == max_points {
//        return 1.0;
//    }
//    // val is in the range (0,1)
//    let val: f64 = count as f64 / max_points as f64;
//    // Values closer to 1 look better, so raise x to the power
//    // of 1/bias_towards_one to get a curve that's biased towards 1
//    // https://www.desmos.com/calculator/ksrfppmuab 
//    return val.powf(1.0/bias_towards_one);
//}

//fn u32_to_argb(i: u32) -> (u8, u8, u8, u8) {
//    let a: u8 = ((i >> 24) & 0x0000_00FF).try_into().unwrap();
//    let r: u8 = ((i >> 16) & 0x0000_00FF).try_into().unwrap();
//    let g: u8 = ((i >>  8) & 0x0000_00FF).try_into().unwrap();
//    let b: u8 = ((i      ) & 0x0000_00FF).try_into().unwrap();
//    
//    return (a, r, g, b);
//}

fn argb_to_u32(a: u8, r: u8, g: u8, b: u8) -> u32 {
    let (a, r, g, b) = (a as u32, r as u32, g as u32, b as u32);
    (a << 24) | (r << 16) | (g << 8) | b
}

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

// TODO plot the keyframes as lines tracing the a,b,c,d parameters along the 
// bottom of the screen
//fn _plot_keyframes(_keyframes: Vec<ParamPosition>, _hud: &mut Vec<u32>) {
//
//}
//fn circ(cx: isize, cy: isize, r: isize, hud: &mut Vec<u32>) {
//    for x in (cx-r)..(cx+r) {
//        for y in (cy-r)..(cy+r) {
//            if (x-cx).pow(2) + (y-cy).pow(2) <= r * r {
//                let i = x as usize + y as usize * WIDTH;
//                hud[i] = argb_to_u32(255, 255, 255, 255);
//            }
//        }
//    }
//}
//
//fn line(x1: usize, y1: usize, x2: usize, y2: usize, hud: &mut Vec<u32>) {
//    let num_blocks = cmp::max(
//        (x1 as isize - x2 as isize).abs(), 
//        (y1 as isize - y2 as isize).abs());
//    for i in 0..num_blocks {
//        let t = i as f64 / num_blocks as f64;
//        let x0 = (x1 as f64 + t * (x2 as f64 - x1 as f64)) as usize;
//        let y0 = (y1 as f64 + t * (y2 as f64 - y1 as f64)) as usize;
//        let i = x0 + y0 * WIDTH;
//        hud[i] = argb_to_u32(255, 255, 255, 255);
//    }
//}

