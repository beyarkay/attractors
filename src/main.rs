mod attractors;
use crate::attractors::*;
extern crate minifb;
use std::cmp;

mod attractors;
use crate::attractors::*;

use rand::Rng;
use minifb::{Key, Window, WindowOptions};

const WIDTH: usize = 800;
const HEIGHT: usize = 800;

fn main() {
    // Write a CliffordAttractor to file
    let params = vec![1.5; 4];
    let mut clifford: CliffordAttractor = CliffordAttractor::new(params);
    let mut x = 0.0;
    let mut y = 0.0;
    clifford.step(&mut x, &mut y, 10);
    clifford.to_file("filename.txt".to_string());

    // Actually draw a Clifford attractor
    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];
    let mut attr_count: Vec<u32> = vec![0; WIDTH * HEIGHT];
    let mut hud: Vec<u32> = vec![0; WIDTH * HEIGHT];
    let _keyframes: Vec<ParamPosition> = vec![
        ParamPosition { a: -1.0, b: -2.0, c: -3.0, d: -4.0 },
        ParamPosition { a:  0.0, b: -3.0, c: -3.0, d:  4.0 },
        ParamPosition { a:  1.0, b: -4.0, c: -3.0, d: -4.0 },
    ];
    let mut show_hud: bool = false;
    let mut max_points: u32 = 0;
    let mut rng = rand::thread_rng();
    let mut param_pos = ParamPosition {
        a: rng.gen_range(-5.0..5.0),
        b: rng.gen_range(-5.0..5.0),
        c: rng.gen_range(-5.0..5.0),
        d: rng.gen_range(-5.0..5.0),
    };

    let mut pos = Position {
        x: 0.0,
        y: 0.0
    };
    // Loop through the attractor a few times to get away from (0,0)
    for _ in 1..20 {
        clifford(&mut pos, &param_pos);
    }

    let mut window = Window::new(
        "Attractors - q to Quit",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
        ).unwrap_or_else(|e| {
        panic!("{}", e);
    });

    // Limit to max ~60 fps update rate
    window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));
    line(20, 20, 200, 200, &mut hud);
    circ(20, 20, 10, &mut hud);
    circ(200, 200, 10, &mut hud);

    while window.is_open() && !window.is_key_down(Key::Q) {
        let min_x: f64 = -1.0 - param_pos.c.abs();
        let max_x: f64 =  1.0 + param_pos.c.abs();
        let min_y: f64 = -1.0 - param_pos.d.abs();
        let max_y: f64 =  1.0 + param_pos.d.abs();
        for _ in 1..50000 {
            clifford(&mut pos, &param_pos);
            let pos_scaled = Position {
                x: (pos.x/(max_x-min_x) * 0.95*WIDTH as f64) + 0.5*WIDTH as f64,
                y: (pos.y/(max_y-min_y) * 0.95*HEIGHT as f64) + 0.5*HEIGHT as f64,
            };
            let i = pos_scaled.x as usize + pos_scaled.y as usize * WIDTH;
            attr_count[i] += 1;
            // Store the maximum number of points in one pixel, so as to scale all
            // other results by that amount
            if attr_count[i] > max_points {
                max_points = attr_count[i];
            }
        }

        for (i, item) in buffer.iter_mut().enumerate() {
            let val: f64 = count_to_brightness(attr_count[i], max_points, 2.5);
            if show_hud {
                let (a, r, g, b) = u32_to_argb(hud[i]);
                let a: f64 = a as f64;
                let r: f64 = r as f64;
                let g: f64 = g as f64;
                let b: f64 = b as f64;
                *item = argb_to_u32(
                    255,
                    (val * 255.0 * (1.0 - a) + r * a) as u8,
                    (val * 255.0 * (1.0 - a) + g * a) as u8,
                    (val * 255.0 * (1.0 - a) + b * a) as u8);
            } else {
                *item = argb_to_u32(
                    255,
                    (val * 255.0) as u8,
                    (val * 255.0) as u8,
                    (val * 255.0) as u8);
            }
        }
        window.get_keys().map(|keys| {
            for t in keys {
                match t {
                    Key::A =>{
                        param_pos.a += 0.1;
                        reset(&mut buffer, &mut attr_count, &mut max_points);
                    },
                    Key::B =>{
                        param_pos.b += 0.1;
                        reset(&mut buffer, &mut attr_count, &mut max_points);
                    },
                    Key::C =>{
                        param_pos.c += 0.1;
                        reset(&mut buffer, &mut attr_count, &mut max_points);
                    },
                    Key::D =>{
                        param_pos.d += 0.1;
                        reset(&mut buffer, &mut attr_count, &mut max_points);
                    },
                    Key::H => {
                        show_hud = !show_hud;
                    },
                    Key::S => {
                        param_pos.a = -1.032;
                        param_pos.b = -1.731;
                        param_pos.c = -0.286;
                        param_pos.d = 2.92;
                    },
                    Key::R => {
                        param_pos.a = rng.gen_range(-5.0..5.0);
                        param_pos.b = rng.gen_range(-5.0..5.0);
                        param_pos.c = rng.gen_range(-5.0..5.0);
                        param_pos.d = rng.gen_range(-5.0..5.0);
                        reset(&mut buffer, &mut attr_count, &mut max_points);
                    },
                    _ => (),
                }
            }
        });
        // We unwrap here as we want this code to exit if it fails. Real applications may want to handle this in a different way
        window.update_with_buffer(&buffer, WIDTH, HEIGHT)
              .unwrap();
    }
}

#[derive(Debug)]
struct ParamPosition { a: f64, b: f64, c: f64, d: f64 }

#[derive(Debug)]
struct Position { x: f64, y: f64 }

fn clifford(pos: &mut Position, param_pos: &ParamPosition) {
    pos.x = (param_pos.a * pos.y).sin() + param_pos.c * (param_pos.a * pos.x).cos();
    pos.y = (param_pos.b * pos.x).sin() + param_pos.d * (param_pos.b * pos.y).cos();
}

fn reset(buffer: &mut Vec<u32>, attr_count: &mut Vec<u32>, max_points: &mut u32) {
    *max_points = 0;
    *attr_count = vec![0; attr_count.len()];
    *buffer = vec![0; buffer.len()];
}

fn count_to_brightness(count: u32, max_points: u32, bias_towards_one: f64) -> f64 {
    // short-circuit for common situations, to avoid floating point division
    if count == 0 {
        return 0.0;
    } else if count == max_points {
        return 1.0;
    }
    // val is in the range (0,1)
    let val: f64 = count as f64 / max_points as f64;
    // Values closer to 1 look better, so raise x to the power
    // of 1/bias_towards_one to get a curve that's biased towards 1
    // https://www.desmos.com/calculator/ksrfppmuab 
    return val.powf(1.0/bias_towards_one);
}

fn u32_to_argb(i: u32) -> (u8, u8, u8, u8) {
    let a: u8 = ((i >> 24) & 0x0000_00FF).try_into().unwrap();
    let r: u8 = ((i >> 16) & 0x0000_00FF).try_into().unwrap();
    let g: u8 = ((i >>  8) & 0x0000_00FF).try_into().unwrap();
    let b: u8 = ((i      ) & 0x0000_00FF).try_into().unwrap();
    
    return (a, r, g, b);
}

fn argb_to_u32(a: u8, r: u8, g: u8, b: u8) -> u32 {
    let (a, r, g, b) = (a as u32, r as u32, g as u32, b as u32);
    (a << 24) | (r << 16) | (g << 8) | b
}

// TODO plot the keyframes as lines tracing the a,b,c,d parameters along the 
// bottom of the screen
fn plot_keyframes(keyframes: Vec<ParamPosition>, hud: &mut Vec<u32>) {
    //for (i, kf) in keyframes.enumerate() {
    //    circ(
    //        (i as f64 / keyframes.len() as f64 * WIDTH as f64) as isize, 
    //        HEIGHT as f64 * 0.9 + 
    //}
}

fn circ(cx: isize, cy: isize, r: isize, hud: &mut Vec<u32>) {
    for x in (cx-r)..(cx+r) {
        for y in (cy-r)..(cy+r) {
            if (x-cx).pow(2) + (y-cy).pow(2) <= r * r {
                let i = x as usize + y as usize * WIDTH;
                hud[i] = argb_to_u32(255, 255, 255, 255);
            }
        }
    }
}

fn line(x1: usize, y1: usize, x2: usize, y2: usize, hud: &mut Vec<u32>) {
    let num_blocks = cmp::max(
        (x1 as isize - x2 as isize).abs(), 
        (y1 as isize - y2 as isize).abs());
    for i in 0..num_blocks {
        let t = i as f64 / num_blocks as f64;
        let x0 = (x1 as f64 + t * (x2 as f64 - x1 as f64)) as usize;
        let y0 = (y1 as f64 + t * (y2 as f64 - y1 as f64)) as usize;
        let i = x0 + y0 * WIDTH;
        hud[i] = argb_to_u32(255, 255, 255, 255);
    }
}

