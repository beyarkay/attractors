#![feature(test)]
extern crate minifb;
extern crate test;
mod attractors;
use std::fs::File;
use std::thread::{sleep, sleep_ms};
use std::time::Duration;
use std::{fmt::Display, fs::OpenOptions, io::BufWriter, path::Path};

use crate::attractors::*;
use image::{ImageBuffer, RgbImage};
use minifb::{clamp, CursorStyle, Key, MouseButton, MouseMode, Window, WindowOptions};
use rand::Rng;
use std::io::{prelude::*, BufReader};

enum IsoPaper {
    A0,
    A1,
    A2,
    A3,
    A4,
    A5,
}

const A0_600_DPI: (usize, usize) = (19866, 28087);
const A1_600_DPI: (usize, usize) = (14043, 19866);
const _A2_600_DPI: (usize, usize) = (9933, 14043);
const A3_600_DPI: (usize, usize) = (7016, 9933);
const _A4_600_DPI: (usize, usize) = (4960, 7016);
const ISO_PAPER_FORMAT: (usize, usize) = (636, 900);

const REELS_WIDTH: usize = 506;
const REELS_HEIGHT: usize = 900;
const MAP_WIDTH: usize = 600;
const MAP_HEIGHT: usize = 600;
const DIAG_WIDTH: usize = 300;
const DIAG_HEIGHT: usize = 100;
const SCREEN_WIDTH: usize = 900;
const SCREEN_HEIGHT: usize = 900;
const WIDTH: usize = ISO_PAPER_FORMAT.0;
const HEIGHT: usize = ISO_PAPER_FORMAT.1;
const FIRST_DRAW_SIZE: usize = 9_000_000;
const MIN_NUM_STEPS: usize = 200_000;

fn main() {
    // Create parameters for the clifford attractor
    let mut delta = 0.01;
    let mut specials = get_specials();
    let mut clifford: CliffordAttractor = CliffordAttractor::new(vec![-1.4, 1.6, 1.0, 0.7]);
    if let Some(ref specials) = specials {
        let mut rng = rand::thread_rng();
        let special_idx = rng.gen_range(0..specials.len());
        clifford.set_params(vec![
            Some(specials[special_idx][0]),
            Some(specials[special_idx][1]),
            Some(specials[special_idx][2]),
            Some(specials[special_idx][3]),
        ]);
    }
    // clifford.to_file(format!(
    //         "cache/clifford/{}-a={}-b={}-c={}-d={}.txt",
    //         CliffordAttractor::NAME, clifford.a, clifford.b, clifford.c, clifford.d
    //         ).to_string());

    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];
    let mut window = Window::new(
        "Strange Attractors (hold esc to exit)",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
    )
    .unwrap_or_else(|e| {
        panic!("{}", e);
    });

    // Each frame is calculated via an exponential decay as
    // `next = noodle_factor * current + (1-noodle_factor) * previous`
    // Values of noodle_factor closer to 0 will 'fade in' or 'blend' the next frame with the
    // current one, reducing stuttering or flashing effects
    let mut noodle_factor = 0.9;

    let commands = vec![
        Command { // j -> a--
            keys: vec![Key::J],
            action: Box::new(|clifford, _buffer, _keys, _lch, _specials, _decay_factor, delta| {
                clifford.set_params(vec![Some(clifford.a - *delta), None, None, None]);
                clifford.reset();
                clifford.step(MIN_NUM_STEPS);
            }),
            description: "u => a -= delta".to_string(),
            enabled: true,
        },
        Command { // k -> b--
            keys: vec![Key::K],
            action: Box::new(|clifford, _buffer, _keys, _lch, _specials, _decay_factor, delta| {
                clifford.set_params(vec![None, Some(clifford.b - *delta), None, None]);
                clifford.reset();
                clifford.step(MIN_NUM_STEPS);
            }),
            description: "i => b -= delta".to_string(),
            enabled: true,
        },
        Command { // l -> c--
            keys: vec![Key::L],
            action: Box::new(|clifford, _buffer, _keys, _lch, _specials, _decay_factor, delta| {
                clifford.set_params(vec![None, None, Some(clifford.c - *delta), None]);
                clifford.reset();
                clifford.step(MIN_NUM_STEPS);
            }),
            description: "l => c -= delta".to_string(),
            enabled: true,
        },
        Command { // ; -> d--
            keys: vec![Key::Semicolon],
            action: Box::new(|clifford, _buffer, _keys, _lch, _specials, _decay_factor, delta| {
                clifford.set_params(vec![None, None, None, Some(clifford.d - *delta)]);
                clifford.reset();
                clifford.step(MIN_NUM_STEPS);
            }),
            description: "p => d -= delta".to_string(),
            enabled: true,
        },
        Command { // u -> a++
            keys: vec![Key::U],
            action: Box::new(|clifford, _buffer, _keys, _lch, _specials, _decay_factor, delta| {
                clifford.set_params(vec![Some(clifford.a + *delta), None, None, None]);
                clifford.reset();
                clifford.step(MIN_NUM_STEPS);
            }),
            description: "u => a += delta".to_string(),
            enabled: true,
        },
        Command { // i -> b++
            keys: vec![Key::I],
            action: Box::new(|clifford, _buffer, _keys, _lch, _specials, _decay_factor, delta| {
                clifford.set_params(vec![None, Some(clifford.b + *delta), None, None]);
                clifford.reset();
                clifford.step(MIN_NUM_STEPS);
            }),
            description: "i => b += delta".to_string(),
            enabled: true,
        },
        Command { // o -> c++
            keys: vec![Key::O],
            action: Box::new(|clifford, _buffer, _keys, _lch, _specials, _decay_factor, delta| {
                clifford.set_params(vec![None, None, Some(clifford.c + *delta), None]);
                clifford.reset();
                clifford.step(MIN_NUM_STEPS);
            }),
            description: "o => c += delta".to_string(),
            enabled: true,
        },
        Command { // p -> d++
            keys: vec![Key::P],
            action: Box::new(|clifford, _buffer, _keys, _lch, _specials, _decay_factor, delta| {
                clifford.set_params(vec![None, None, None, Some(clifford.d + *delta)]);
                clifford.reset();
                clifford.step(MIN_NUM_STEPS);
            }),
            description: "p => d += delta".to_string(),
            enabled: true,
        },
        Command { // Hue Intercept
            keys: vec![Key::E],
            action: Box::new(|clifford, _buffer, keys, lch, _specials, _decay_factor, _delta| {
                let sign = if keys.contains(&Key::LeftShift) { -1.0 } else { 1.0 };
                lch.hue_intercept += 0.01 * sign;
                println!("{:#}", lch);
                clifford.step(1);
            }),
            description: "Increase or decrease the LCH hue intercept by 0.01".to_string(),
            enabled: true,
        },
        Command { // Reset and randomise
            keys: vec![Key::R],
            action: Box::new(|clifford, buffer, keys, _lch, specials, _decay_factor, _delta| {
                for item in buffer.iter_mut() { *item = 0; }
                // If r => Choose random from specials
                if !keys.contains(&Key::LeftShift) {
                    if let Some(specials) = specials {
                        let mut rng = rand::thread_rng();
                        let special_idx = rng.gen_range(0..specials.len());
                        clifford.set_params(vec![
                                            Some(specials[special_idx][0]),
                                            Some(specials[special_idx][1]),
                                            Some(specials[special_idx][2]),
                                            Some(specials[special_idx][3]),
                        ]);
                    }
                } else {
                    // If R => Choose random range -4, 4
                    let mut rng = rand::thread_rng();
                    clifford.set_params(vec![
                                        Some(rng.gen_range(-4.0..4.0)),
                                        Some(rng.gen_range(-4.0..4.0)),
                                        Some(rng.gen_range(-4.0..4.0)),
                                        Some(rng.gen_range(-4.0..4.0)),
                    ]);
                };
                clifford.reset();
                clifford.step(MIN_NUM_STEPS);
            }),
            description: "Randomize the Clifford parameters and re-run the attractor with these new parameters".to_string(),
            enabled: true,
        },
        Command { // Print to disc
            keys: vec![Key::S],
            action: Box::new(|clifford, _buffer, keys, lch, _specials, _decay_factor, _delta| {
                let filename = format!("cache/clifford/a={:.6}_b={:.6}_c={:.6}_d={:.6}_iters={}.png", clifford.a, clifford.b, clifford.c, clifford.d, clifford.history.len());
                print!("Saving data to {}", filename);
                let size = if keys.contains(&Key::LeftShift) { A0_600_DPI } else { A3_600_DPI };
                let mut image: RgbImage = ImageBuffer::new(size.0.try_into().unwrap(), size.1.try_into().unwrap());
                while clifford.history.len() < 40_000_000 {
                    print!(".");
                    clifford.step(5_000_000);
                }
                let densities = clifford.get_densities_with_border(size.0 as usize, size.1 as usize, 0.05);
                for (i, val) in densities.iter().enumerate() {
                    let packed = hsla_to_u32(
                        val * lch.hue_slope + lch.hue_intercept,
                        val * lch.chroma_slope + lch.chroma_intercept,
                        // The lightness is inversely proportional to the size of of the grid, so
                        // adjust the lightness power accordingly
                        val.powf(0.1) * lch.light_slope + lch.light_intercept,
                        0.0,
                    );
                    let x = i % size.0 as usize;
                    let y = i / size.0 as usize;
                    let (_a, r, g, b) = u32_to_argb(packed);
                    image.put_pixel(x as u32, y as u32, image::Rgb([r, g, b]));
                }
                image.save(filename).unwrap();
                println!("done");
            }),
            description: "Save the attractor in high resolution to disc as png".to_string(),
            enabled: true,
        },
        Command { // Change from black bg to white bg
            keys: vec![Key::B],
            action: Box::new(|clifford, _buffer, _keys, lch, _specials, _decay_factor, _delta| {
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
            description: "Change between black and white backgrounds".to_string(),
            enabled: true,
        },
        Command { // Mark the location as 'special'
            keys: vec![Key::M],
            action: Box::new(|clifford, _buffer, _keys, _lch, specials, _decay_factor, _delta| {
                let filename = "cache/clifford/special.txt";
                if let Some(specials) = specials {
                    specials.push(vec![clifford.a, clifford.b, clifford.c, clifford.d]);
                }

                let file;
                if !Path::new(filename).exists() {
                    // If the file doesn't exist, create it and write the csv header line
                    file = OpenOptions::new()
                        .create_new(true)
                        .write(true)
                        .append(true)
                        .open(filename)
                        .expect("Failed to create new file");
                } else {
                    file = OpenOptions::new().append(true).open(filename).expect("Couldn't open file for appending");
                }
                let file_read = File::open(filename).expect("file not found!");
                let reader = BufReader::new(file_read);
                let to_add = format!("a={},b={},c={},d={}", clifford.a, clifford.b, clifford.c, clifford.d);
                let mut already_in_file = false;
                for line in reader.lines() {
                    if line.expect("Failed to unwrap line of special.txt").contains(&to_add) {
                        already_in_file = true;
                    }
                }
                if !already_in_file {
                    let mut file = BufWriter::new(file);
                    writeln!(file, "{}", to_add).expect("Failed to write to file");
                    file.flush().expect("Failed to flush the BufWriter");

                    println!("Marked location as special: a={:<10.4}b={:<10.4}c={:<10.4}d={:<10.4}", clifford.a, clifford.b, clifford.c, clifford.d);
                }
                sleep(Duration::from_millis(10));
            }),
            description: "Mark a set of parameters as 'special' and save them to a file for future use".to_string(),
            enabled: true,
        },
        Command { // Change how quickly images blend together (helps with image flickering)
            keys: vec![Key::N],
            action: Box::new(|_clifford, _buffer, keys, _lch, _specials, noodle_factor, _delta| {
                let sign = if keys.contains(&Key::LeftShift) { -1.0 } else { 1.0 };
                *noodle_factor = f64::min(1.0, f64::max(0.05, *noodle_factor + sign * 0.05));
                println!("noodle_factor: {}", noodle_factor);
            }),
            description: "Change how quickly one attractor merges to another (helps with photosensitive epilepsy)".to_string(),
            enabled: true,
        },
        Command { // Change the amount by which to step the parameters
            keys: vec![Key::T],
            action: Box::new(|_clifford, _buffer, keys, _lch, _specials, _noodle_factor, delta| {
                let sign = if keys.contains(&Key::LeftShift) { -1 } else { 1 };
                *delta = *delta * 10.0_f64.powi(sign); // Either multiply or divide by 10
                println!("delta: {delta}");
                sleep(Duration::from_millis(10));
            }),
            description: "Change how quickly the parameter values are changed".to_string(),
            enabled: true,
        },
        ];
    println!("=== List of Commands ===");
    for command in commands.iter() {
        println!(
            "`{:?}` => {} (enabled: {})",
            command.keys, command.description, command.enabled
        );
    }

    // These parameters have been manually tuned
    let mut lch = LchParams {
        light_intercept: 0.0, // no touchie
        light_slope: 1.0,
        chroma_intercept: 1.5,
        chroma_slope: 0.2,
        hue_intercept: 0.45,
        hue_slope: 0.15, // values over 0.5 give a bit of a blowout effect
    };

    let mut map_window = Window::new(
        "Map",
        MAP_WIDTH,
        MAP_HEIGHT,
        WindowOptions {
            ..WindowOptions::default()
        },
    )
    .unwrap();
    map_window.set_position(0, 0);

    // let mut diagnostics = Window::new(
    //     "Diagnostics",
    //     DIAG_WIDTH,
    //     DIAG_HEIGHT,
    //     WindowOptions {
    //         ..WindowOptions::default()
    //     },
    // ).unwrap();
    // diagnostics.set_position(0, 65 + MAP_HEIGHT as isize);
    window.set_position(MAP_WIDTH as isize, 0);

    clifford.step(MIN_NUM_STEPS);
    let mut densities;
    let mut prev_densities = vec![0f64; WIDTH * HEIGHT];
    // let mut diag_buf = vec![0u32; DIAG_WIDTH * DIAG_HEIGHT];
    let mut map_buf = vec![0u32; MAP_WIDTH * MAP_HEIGHT];
    while window.is_open() && !window.is_key_down(Key::Escape) {
        // Then use those generated points to draw onto the buffer in
        // the appropriate spaces
        if clifford.history.len() < 20_000_000 {
            clifford.step(MIN_NUM_STEPS);
        }
        densities = clifford.get_densities_with_border(WIDTH, HEIGHT, 0.05);
        let avg_density = densities.iter().sum::<f64>() / densities.len() as f64;
        for (i, item) in buffer.iter_mut().enumerate() {
            prev_densities[i] =
                noodle_factor * densities[i] + (1.0 - noodle_factor) * prev_densities[i];
            *item = hsla_to_u32(
                (prev_densities[i]) * lch.hue_slope + lch.hue_intercept,
                (prev_densities[i]) * lch.chroma_slope + lch.chroma_intercept,
                (prev_densities[i]).powf(0.3) * lch.light_slope + lch.light_intercept,
                0.0,
            );
        }
        if map_window.is_open() {
            let mouse_pos = map_window.get_mouse_pos(MouseMode::Discard);
            let new_params = update_map(
                &mut map_buf,
                &clifford,
                &specials,
                &mouse_pos,
                map_window.get_mouse_down(MouseButton::Left),
                &mut map_window,
            );
            map_window
                .update_with_buffer(&map_buf, MAP_WIDTH, MAP_HEIGHT)
                .unwrap();
            // Only update the attractor if >0 of the parameters have changed
            if new_params.iter().any(|p| p.is_some()) {
                clifford.set_params(new_params);
                clifford.reset();
                clifford.step(MIN_NUM_STEPS);
            }
        }
        let wind_keys = window.get_keys();
        for cmd in &commands {
            // check if the currently pressed keys match any of the commands' required keys
            if cmd.enabled && cmd.keys.iter().all(|k| wind_keys.contains(k)) {
                (cmd.action)(
                    &mut clifford,
                    &mut buffer,
                    &wind_keys,
                    &mut lch,
                    &mut specials,
                    &mut noodle_factor,
                    &mut delta,
                );
            }
        }
        window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();

        // if diagnostics.is_open() {
        //     update_diagnostics(&mut diag_buf, &clifford, avg_density);
        //     diagnostics.update_with_buffer(&diag_buf, DIAG_WIDTH, DIAG_HEIGHT).unwrap();
        // }
    }
}

fn update_map(
    map_buf: &mut Vec<u32>,
    clifford: &CliffordAttractor,
    specials: &Option<Vec<Vec<f64>>>,
    mouse_pos: &Option<(f32, f32)>,
    mouse_down: bool,
    map_window: &mut Window,
) -> Vec<Option<f64>> {
    let color_axes = argb_to_u32(0, 40, 40, 40);
    let color_specials = argb_to_u32(0, 50, 50, 50);
    let color_specials_exact = argb_to_u32(0, 0, 255, 0);
    let color_crosshairs = argb_to_u32(0, 150, 150, 150);
    let color_crosshairs_mouse = argb_to_u32(0, 150, 150, 150);

    let axes = vec![
        (clifford.a, clifford.b),
        (clifford.c, clifford.b),
        (clifford.a, clifford.d),
        (clifford.c, clifford.d),
    ];
    // Erase everything, we want to start from a blank canvas
    map_buf.fill(0);
    let mut returner = vec![None; 4];
    // Draw the axes, and the specially marked points on those axes
    for (plt_idx, ax) in axes.iter().enumerate() {
        // Figure out the topleft pixel coordinate for the current axis
        let topleft = (
            (0.5 * MAP_WIDTH as f64 * if plt_idx % 2 == 1 { 1.0 } else { 0.0 }) as usize,
            (0.5 * MAP_HEIGHT as f64 * if plt_idx / 2 == 1 { 1.0 } else { 0.0 }) as usize,
        );
        let botright = (
            (topleft.0 as f64 + 0.5 * MAP_WIDTH as f64) as usize,
            (topleft.1 as f64 + 0.5 * MAP_HEIGHT as f64) as usize,
        );

        // Calculate the current x and y position
        let x = from_range_to_domain(
            ax.0,
            -5.0,
            5.0,
            topleft.0 as f64,
            topleft.0 as f64 + 0.5 * MAP_WIDTH as f64,
        );
        let y = from_range_to_domain(
            ax.1,
            -5.0,
            5.0,
            topleft.1 as f64,
            topleft.1 as f64 + 0.5 * MAP_WIDTH as f64,
        );

        // Draw the axes
        let zero_x = from_range_to_domain(
            0.0,
            -5.0,
            5.0,
            topleft.0 as f64,
            topleft.0 as f64 + 0.5 * MAP_WIDTH as f64,
        ) as usize;
        let zero_y = from_range_to_domain(
            0.0,
            -5.0,
            5.0,
            topleft.1 as f64,
            topleft.1 as f64 + 0.5 * MAP_WIDTH as f64,
        ) as usize;
        let x_border = 0.01 * MAP_WIDTH as f64;
        for x in (x_border as usize)..((0.5 * MAP_WIDTH as f64 - x_border) as usize) {
            map_buf[xy2idx(x + topleft.0, zero_y, MAP_WIDTH, MAP_HEIGHT)] = color_axes;
        }
        let y_border = 0.01 * MAP_HEIGHT as f64;
        for y in (y_border as usize)..((0.5 * MAP_HEIGHT as f64 - y_border) as usize) {
            map_buf[xy2idx(zero_x, y + topleft.1, MAP_WIDTH, MAP_HEIGHT)] = color_axes;
        }

        // Mark all the special points on the map
        if let Some(specials) = specials {
            for special in specials {
                let mx = from_range_to_domain(
                    // x component is either a (plots 0 and 2) or c (plots 1 and 3)
                    special[if plt_idx % 2 == 0 { 0 } else { 2 }],
                    -5.0,
                    5.0,
                    topleft.0 as f64,
                    topleft.0 as f64 + 0.5 * MAP_WIDTH as f64,
                );
                let my = from_range_to_domain(
                    // y component is either b (plots 0 and 1) or d (plots 2 and 3)
                    special[if plt_idx / 2 == 0 { 1 } else { 3 }],
                    -5.0,
                    5.0,
                    topleft.1 as f64,
                    topleft.1 as f64 + 0.5 * MAP_WIDTH as f64,
                );

                // 0:(a,b) 1:(c,b)
                // 2:(a,d) 3:(c,d)
                let opposite_plot_idx = 3 - plt_idx;

                let x_opp_special = special[if opposite_plot_idx % 2 == 0 { 0 } else { 2 }];
                let x_opp_actual = axes[opposite_plot_idx].0;

                let y_opp_special = special[if opposite_plot_idx / 2 == 0 { 1 } else { 3 }];
                let y_opp_actual = axes[opposite_plot_idx].1;

                let x_dist = (x_opp_actual - x_opp_special).abs();
                let y_dist = (y_opp_actual - y_opp_special).abs();

                let max_dist = 1.5;
                if x_dist < 0.1 && y_dist < 0.1 {
                    // If the attractor is basically exactly on the mark, colour it green
                    map_buf[xy2idx(mx as usize, my as usize, MAP_WIDTH, MAP_HEIGHT)] =
                        color_specials_exact;
                } else if x_dist < max_dist && y_dist < max_dist {
                    // Otherwise, if the attractor is close but not exact, colour it closer to
                    // white than grey
                    let normalised_dist =
                        (x_dist * x_dist + y_dist * y_dist).sqrt() / (2_f64.sqrt() * max_dist);
                    let amount_to_add = (200.0 - 200.0 * normalised_dist) as u8;
                    map_buf[xy2idx(mx as usize, my as usize, MAP_WIDTH, MAP_HEIGHT)] = argb_to_u32(
                        0,
                        50 + amount_to_add,
                        50 + amount_to_add,
                        50 + amount_to_add,
                    );
                } else {
                    map_buf[xy2idx(mx as usize, my as usize, MAP_WIDTH, MAP_HEIGHT)] =
                        color_specials;
                }
            }
        }

        // Draw the current position, and cross hairs lines marking it's position
        for delta in ((-30)..30).step_by(3) {
            // But leave the actual centre point unmarked
            if (delta as i8).abs() < 10 {
                continue;
            }
            map_buf[xy2idx(
                (x as isize + delta + MAP_WIDTH as isize) as usize % MAP_WIDTH,
                y as usize,
                MAP_WIDTH,
                MAP_HEIGHT,
            )] = color_crosshairs;
            map_buf[xy2idx(
                x as usize,
                (y as isize + delta + MAP_HEIGHT as isize) as usize % MAP_HEIGHT,
                MAP_WIDTH,
                MAP_HEIGHT,
            )] = color_crosshairs;
        }

        // If the user clicks on this set of axes, change the parameters
        if let Some((mousex, mousey)) = mouse_pos {
            // Figure out if the user's even clicking on the current plot
            let mouse_in_curr_plot_x = topleft.0 < *mousex as usize && *mousex < botright.0 as f32;
            let mouse_in_curr_plot_y = topleft.1 < *mousey as usize && *mousey < botright.1 as f32;
            if mouse_down && mouse_in_curr_plot_x && mouse_in_curr_plot_y {
                // Use a cross-hair cursor
                map_window.set_cursor_style(CursorStyle::Crosshair);
                // Convert pixel-coordinates to parameter-coordinates
                let param_x = from_range_to_domain(
                    *mousex as f64,
                    topleft.0 as f64,
                    botright.0 as f64,
                    -5.0,
                    5.0,
                );
                let param_y = from_range_to_domain(
                    *mousey as f64,
                    topleft.1 as f64,
                    botright.1 as f64,
                    -5.0,
                    5.0,
                );
                // Resolve parameter values to the correct attractor parameters based on which plot
                // we're currently resolving
                let a = if plt_idx == 0 || plt_idx == 2 {
                    Some(param_x)
                } else {
                    None
                };
                let b = if plt_idx == 0 || plt_idx == 1 {
                    Some(param_y)
                } else {
                    None
                };
                let c = if plt_idx == 1 || plt_idx == 3 {
                    Some(param_x)
                } else {
                    None
                };
                let d = if plt_idx == 2 || plt_idx == 3 {
                    Some(param_y)
                } else {
                    None
                };
                returner = vec![a, b, c, d];
            } else {
                // Reset the cursor to not be gone
                map_window.set_cursor_style(CursorStyle::Arrow);
            }
        }
    }

    // Draw cross-hairs for the mouse's current position
    if let Some((mousex, mousey)) = mouse_pos {
        for delta in (-10)..10 {
            map_buf[xy2idx(
                (*mousex as isize + delta + MAP_WIDTH as isize) as usize % MAP_WIDTH,
                *mousey as usize,
                MAP_WIDTH,
                MAP_HEIGHT,
            )] = color_crosshairs_mouse;
            map_buf[xy2idx(
                *mousex as usize,
                (*mousey as isize + delta + MAP_HEIGHT as isize) as usize % MAP_HEIGHT,
                MAP_WIDTH,
                MAP_HEIGHT,
            )] = color_crosshairs_mouse;
        }
    }
    return returner;
}

fn xy2idx(x: usize, y: usize, width: usize, height: usize) -> usize {
    return usize::min(usize::max(0, y), height - 1) * width
        + usize::min(usize::max(0, x), width - 1);
}

fn from_range_to_domain(
    x: f64,
    lower_from: f64,
    upper_from: f64,
    lower_to: f64,
    upper_to: f64,
) -> f64 {
    return ((clamp(lower_from, x, upper_from) - lower_from) / (upper_from - lower_from))
        * (upper_to - lower_to)
        + lower_to;
}

// fn update_diagnostics(diag_buf: &mut Vec<u32>, clifford: &CliffordAttractor, avg_density: f64) {
//     for param in clifford.param_history.iter() {
//         let a_in_bounds = -5.0 <= param[0] && param[0] < 5.0;
//         let b_in_bounds = -5.0 <= param[1] && param[1] < 5.0;
//         let c_in_bounds = -5.0 <= param[2] && param[2] < 5.0;
//         let d_in_bounds = -5.0 <= param[3] && param[3] < 5.0;
//         if  a_in_bounds && b_in_bounds && c_in_bounds && d_in_bounds {
//             let a = (param[0] + 5.0) / 10.0 * 100.0; // horizontal axis
//             let b = (param[1] + 5.0) / 10.0 * 100.0; // vertical axis
//             let c = ((param[2] + 5.0) / 10.0 * 200.0 + 50.0) as u8; // Red colour axis
//             let d = ((param[3] + 5.0) / 10.0 * 200.0 + 50.0) as u8; // Green color axis
//             let pos = b as usize * DIAG_WIDTH + a as usize;
//             diag_buf[pos] = argb_to_u32(0, c, d, 255);
//         }
//     }
//     let x = DIAG_WIDTH / 3 + (clifford.param_history.len() % ((DIAG_WIDTH * 2) / 3));
//     for y in 0..DIAG_HEIGHT {
//         let pos = y * DIAG_WIDTH + x as usize;
//         diag_buf[pos] = argb_to_u32(0, 0, 0, 0);
//     }
//     // Raise avg_density to the power of 0.3 because there are _loads_ of small values
//     // (1e-3 < value < 1e-1) which are still meaningful but get lost
//     let y = (avg_density.powf(0.3) * DIAG_HEIGHT as f64 ) as usize;
//     let pos = y * DIAG_WIDTH + x as usize;
//     diag_buf[pos] = argb_to_u32(0, 255, 255, 255);
// }

fn u32_to_argb(packed: u32) -> (u8, u8, u8, u8) {
    let a = ((0xFF_00_00_00 & packed) >> 24) as u8;
    let r = ((0x00_FF_00_00 & packed) >> 16) as u8;
    let g = ((0x00_00_FF_00 & packed) >> 8) as u8;
    let b = (0x00_00_00_FF & packed) as u8;
    return (a, r, g, b);
}

fn get_specials() -> Option<Vec<Vec<f64>>> {
    let filename = "cache/clifford/special.txt";

    if Path::new(filename).exists() {
        let file_read = File::open(filename).expect("file not found!");
        let reader = BufReader::new(file_read);
        let specials = reader
            .lines()
            .map(|l| {
                l.unwrap()
                    .clone()
                    .split(",")
                    .map(|s| {
                        *(&s[2..]
                            .to_owned()
                            .parse::<f64>()
                            .expect("Couldn't parse specials.txt line"))
                    })
                    .collect()
            })
            .collect();
        return Some(specials);
    }
    return None;
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
            if t < 0.0 {
                t += 1.0
            };
            if t > 1.0 {
                t -= 1.0
            };
            // I've got no clue how this works
            if t < 1.0 / 6.0 {
                return p + (q - p) * 6.0 * t;
            };
            if t < 1.0 / 2.0 {
                return q;
            };
            if t < 2.0 / 3.0 {
                return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
            };
            return p;
        }

        // I've got no clue how this works
        let q = if l < 0.5 {
            l * (1.0 + s)
        } else {
            l + s - l * s
        };
        // I've got no clue how this works
        let p = 2.0 * l - q;
        r = (255.0 * hue_to_rgb_floats(p, q, h + 1.0 / 3.0)) as u8;
        g = (255.0 * hue_to_rgb_floats(p, q, h)) as u8;
        b = (255.0 * hue_to_rgb_floats(p, q, h - 1.0 / 3.0)) as u8;
    }
    return argb_to_u32(a, r, g, b);
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
    action: Box<
        dyn Fn(
            &mut CliffordAttractor,
            &mut Vec<u32>,
            &Vec<Key>,
            &mut LchParams,
            &mut Option<Vec<Vec<f64>>>,
            &mut f64,
            &mut f64,
        ) -> (),
    >,
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
