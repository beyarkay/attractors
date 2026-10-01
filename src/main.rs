use std::env;
use std::fmt::Display;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use attractors::render::{blend_and_colour, save_png};
use attractors::*;
use minifb::{CursorStyle, Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};
use rand::Rng;

const A0_600_DPI: (usize, usize) = (19866, 28087);
const A1_600_DPI: (usize, usize) = (14043, 19866);
const A2_600_DPI: (usize, usize) = (9933, 14043);
const A3_600_DPI: (usize, usize) = (7016, 9933);
const A4_600_DPI: (usize, usize) = (4960, 7016);
const A5_600_DPI: (usize, usize) = (3506, 4960);
const ISO_PAPER_FORMAT: (usize, usize) = (636, 900);

const MAP_WIDTH: usize = 600;
const MAP_HEIGHT: usize = 600;
/// The map shows each parameter in [-MAP_LIMIT, MAP_LIMIT]
const MAP_LIMIT: f64 = 5.0;
const WIDTH: usize = ISO_PAPER_FORMAT.0;
const HEIGHT: usize = ISO_PAPER_FORMAT.1;
const MIN_NUM_STEPS: usize = 500_000;
const MAX_PREVIEW_STEPS: usize = 10_000_000;
const DEFAULT_POWER: f64 = 0.15;

/// The attractor the explorer and headless renderer use
type Explored = CliffordAttractor;

/// Where images and `special.txt` for this kind of attractor live
fn output_dir<A: Attractor>() -> PathBuf {
    Path::new("cache").join(A::NAME)
}

fn specials_path<A: Attractor>() -> PathBuf {
    output_dir::<A>().join("special.txt")
}

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.len() {
        1 => run_interactively::<Explored>(),
        2 | 3 => run_headless::<Explored>(&args[1], args.get(2).map_or("A0", String::as_str)),
        _ => println!(
            "Usage: attractors [<file of `a=..,b=..,c=..,d=..` lines> [A0|A1|A2|A3|A4|A5]]"
        ),
    }
}

fn paper_size(name: &str) -> Option<(usize, usize)> {
    match name.to_uppercase().as_str() {
        "A0" => Some(A0_600_DPI),
        "A1" => Some(A1_600_DPI),
        "A2" => Some(A2_600_DPI),
        "A3" => Some(A3_600_DPI),
        "A4" => Some(A4_600_DPI),
        "A5" => Some(A5_600_DPI),
        _ => None,
    }
}

fn run_headless<A: Attractor>(filename: &str, paper: &str) {
    let Some(size) = paper_size(paper) else {
        eprintln!("Unknown paper size `{paper}`, expected one of A0 to A5");
        return;
    };
    let contents = fs::read_to_string(filename).expect("Something went wrong reading the file");
    let lines: Vec<_> = contents
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty() && !l.starts_with('#'))
        .collect();
    println!("About to create {} attractors and save them", lines.len());

    for (i, line) in lines {
        let Some(params) = parse_params(line) else {
            eprintln!("{filename}:{}: couldn't parse `{line}`", i + 1);
            continue;
        };
        let attractor = A::new(params);
        match save_png(
            &attractor,
            size,
            &View::default(),
            DEFAULT_POWER,
            &output_dir::<A>(),
        ) {
            Ok(path) => println!("Saved {}", path.display()),
            Err(e) => eprintln!("Failed to save `{line}`: {e}"),
        }
    }
}

struct State<A> {
    attractor: A,
    sampler: Sampler,
    hist: Histogram,
    view: View,
    specials: Vec<[f64; 4]>,
    /// Each frame is `noodle_factor * current + (1 - noodle_factor) * previous`, so values
    /// closer to 0 blend attractors into each other, reducing flashing
    noodle_factor: f64,
    /// How much each key press changes a parameter
    delta: f64,
    /// Densities are raised to this power before colouring
    power: f64,
}

impl<A: Attractor + Display> State<A> {
    fn restart(&mut self) {
        self.hist.clear();
        self.sampler
            .step(&self.attractor, MIN_NUM_STEPS, &mut self.hist, &self.view);
    }

    fn set_params(&mut self, params: [Option<f64>; 4]) {
        self.attractor.set_params(params);
        println!("{:#}", self.attractor);
        self.sampler.reset();
        self.restart();
    }

    fn nudge(&mut self, idx: usize, sign: f64) {
        let mut params = [None; 4];
        params[idx] = Some(self.attractor.params()[idx] + sign * self.delta);
        self.set_params(params);
    }

    fn set_view(&mut self, view: View) {
        self.view = view;
        self.restart();
    }
}

struct Command<A> {
    key: Key,
    /// Whether holding the key keeps triggering the action
    repeat: bool,
    description: &'static str,
    enabled: bool,
    /// Called with whether either Shift key is held
    action: fn(&mut State<A>, bool),
}

fn sign(shift: bool) -> f64 {
    if shift {
        -1.0
    } else {
        1.0
    }
}

fn nudge<A: Attractor + Display, const IDX: usize, const UP: bool>(s: &mut State<A>, _: bool) {
    s.nudge(IDX, if UP { 1.0 } else { -1.0 });
}

fn nudge_command<A>(
    key: Key,
    action: fn(&mut State<A>, bool),
    description: &'static str,
) -> Command<A> {
    Command {
        key,
        repeat: true,
        description,
        enabled: true,
        action,
    }
}

fn commands<A: Attractor + Display>() -> Vec<Command<A>> {
    vec![
        nudge_command(Key::J, nudge::<A, 0, false>, "a -= delta"),
        nudge_command(Key::K, nudge::<A, 1, false>, "b -= delta"),
        nudge_command(Key::L, nudge::<A, 2, false>, "c -= delta"),
        nudge_command(Key::Semicolon, nudge::<A, 3, false>, "d -= delta"),
        nudge_command(Key::U, nudge::<A, 0, true>, "a += delta"),
        nudge_command(Key::I, nudge::<A, 1, true>, "b += delta"),
        nudge_command(Key::O, nudge::<A, 2, true>, "c += delta"),
        nudge_command(Key::P, nudge::<A, 3, true>, "d += delta"),
        Command {
            key: Key::R,
            repeat: false,
            enabled: true,
            description: "Jump to a random special attractor (Shift: random parameters in [-4, 4])",
            action: |s, shift| {
                let mut rng = rand::thread_rng();
                let params = if shift || s.specials.is_empty() {
                    [(); 4].map(|_| rng.gen_range(-4.0..4.0))
                } else {
                    s.specials[rng.gen_range(0..s.specials.len())]
                };
                s.view = View::default();
                s.set_params(params.map(Some));
            },
        },
        Command {
            key: Key::S,
            repeat: false,
            enabled: true,
            description: "Save the attractor as an A3 png (Shift: A0)",
            action: |s, shift| {
                let size = if shift { A0_600_DPI } else { A3_600_DPI };
                match save_png(&s.attractor, size, &s.view, s.power, &output_dir::<A>()) {
                    Ok(path) => println!("Saved {}", path.display()),
                    Err(e) => eprintln!("Failed to save: {e}"),
                }
            },
        },
        Command {
            key: Key::M,
            repeat: false,
            enabled: true,
            description: "Mark the parameters as special, saving them for future use",
            action: |s, _| mark_special(s),
        },
        Command {
            key: Key::N,
            repeat: true,
            enabled: true,
            description: "Blend attractors into each other less (Shift: more, helps with photosensitive epilepsy)",
            action: |s, shift| {
                s.noodle_factor = (s.noodle_factor + sign(shift) * 0.05).clamp(0.05, 1.0);
                println!("noodle_factor: {:.2}", s.noodle_factor);
            },
        },
        Command {
            key: Key::T,
            repeat: false,
            enabled: true,
            description: "Multiply delta by 10 (Shift: divide by 10)",
            action: |s, shift| {
                s.delta *= 10.0_f64.powf(sign(shift));
                println!("delta: {}", s.delta);
            },
        },
        Command {
            key: Key::D,
            repeat: true,
            enabled: true,
            description: "Increase the colour power by delta, so dim pixels get darker (Shift: decrease)",
            action: |s, shift| {
                s.power = (s.power + sign(shift) * s.delta).clamp(0.01, 1.0);
                println!("power: {:.4}", s.power);
            },
        },
        Command {
            key: Key::Z,
            repeat: true,
            enabled: true,
            description: "Zoom in (Shift: zoom out)",
            action: |s, shift| {
                let zoom = (s.view.zoom + sign(shift) * s.delta * 2.0).clamp(0.01, 100.0);
                println!("Zoomed in at {zoom:.4}");
                s.set_view(View { zoom, ..s.view });
            },
        },
        Command {
            key: Key::H,
            repeat: true,
            enabled: false,
            description: "Pan right (Shift: left)",
            action: |s, shift| {
                let hpan = (s.view.hpan + sign(shift) * s.delta * 2.0 / s.view.zoom)
                    .clamp(-1000.0, 1000.0);
                println!("Horizontal pan is {hpan:.4}");
                s.set_view(View { hpan, ..s.view });
            },
        },
        Command {
            key: Key::V,
            repeat: true,
            enabled: false,
            description: "Pan down (Shift: up)",
            action: |s, shift| {
                let vpan = (s.view.vpan + sign(shift) * s.delta * 2.0 / s.view.zoom)
                    .clamp(-1000.0, 1000.0);
                println!("Vertical pan is {vpan:.4}");
                s.set_view(View { vpan, ..s.view });
            },
        },
        Command {
            key: Key::Y,
            repeat: true,
            enabled: true,
            description: "Run more simulation steps",
            action: |s, _| {
                let steps = ((1_000_000.0 * s.delta) as usize).clamp(1_000, 100_000_000);
                s.sampler.step(&s.attractor, steps, &mut s.hist, &s.view);
                println!("Currently at {} steps", s.hist.steps);
            },
        },
        Command {
            key: Key::X,
            repeat: false,
            enabled: true,
            description: "Reset zoom and pan",
            action: |s, _| {
                println!("Reset zoom and pan");
                s.set_view(View::default());
            },
        },
    ]
}

fn run_interactively<A: Attractor + Display>() {
    let specials = load_specials::<A>();
    let params = if specials.is_empty() {
        A::DEFAULT_PARAMS
    } else {
        specials[rand::thread_rng().gen_range(0..specials.len())]
    };
    let mut state = State {
        attractor: A::new(params),
        sampler: Sampler::default(),
        hist: Histogram::new(WIDTH, HEIGHT),
        view: View::default(),
        specials,
        noodle_factor: 0.9,
        delta: 0.1,
        power: DEFAULT_POWER,
    };
    println!("{:#}", state.attractor);

    let commands = commands();
    println!("=== List of Commands ===");
    for command in commands.iter().filter(|c| c.enabled) {
        println!("`{:?}` => {}", command.key, command.description);
    }

    let mut window = Window::new(
        "Strange Attractors (hold esc to exit)",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
    )
    .unwrap_or_else(|e| panic!("{}", e));
    let mut map_window =
        Window::new("Map", MAP_WIDTH, MAP_HEIGHT, WindowOptions::default()).unwrap();
    map_window.set_position(0, 0);
    window.set_position(MAP_WIDTH as isize, 0);

    state.restart();
    let mut buffer = vec![0u32; WIDTH * HEIGHT];
    let mut prev_densities = vec![0f64; WIDTH * HEIGHT];
    let mut map_buf = vec![0u32; MAP_WIDTH * MAP_HEIGHT];
    while window.is_open() && !window.is_key_down(Key::Escape) {
        if state.hist.steps < MAX_PREVIEW_STEPS {
            state.sampler.step(
                &state.attractor,
                MIN_NUM_STEPS,
                &mut state.hist,
                &state.view,
            );
        }
        blend_and_colour(
            &state.hist,
            &mut prev_densities,
            state.noodle_factor,
            state.power,
            &mut buffer,
        );

        if map_window.is_open() {
            let new_params = update_map(
                &mut map_buf,
                &state.attractor.params(),
                &state.specials,
                map_window.get_mouse_pos(MouseMode::Discard),
                map_window.get_mouse_down(MouseButton::Left),
                &mut map_window,
            );
            map_window
                .update_with_buffer(&map_buf, MAP_WIDTH, MAP_HEIGHT)
                .unwrap();
            if new_params.iter().any(|p| p.is_some()) {
                state.set_params(new_params);
            }
        }

        let shift = window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift);
        let repeating = window.get_keys_pressed(KeyRepeat::Yes);
        let once = window.get_keys_pressed(KeyRepeat::No);
        for cmd in commands.iter().filter(|c| c.enabled) {
            let pressed = if cmd.repeat { &repeating } else { &once };
            if pressed.contains(&cmd.key) {
                (cmd.action)(&mut state, shift);
            }
        }
        window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
    }
}

fn load_specials<A: Attractor>() -> Vec<[f64; 4]> {
    let path = specials_path::<A>();
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return vec![],
        Err(e) => {
            eprintln!("Couldn't read {}: {e}", path.display());
            return vec![];
        }
    };
    contents
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .filter_map(|(i, line)| {
            let params = parse_params(line);
            if params.is_none() {
                eprintln!(
                    "{}:{}: skipping malformed line `{line}`",
                    path.display(),
                    i + 1
                );
            }
            params
        })
        .collect()
}

fn mark_special<A: Attractor>(state: &mut State<A>) {
    let params = state.attractor.params();
    if state.specials.contains(&params) {
        println!("Already marked as special: {}", format_params(params));
        return;
    }
    match append_special::<A>(params) {
        Ok(()) => {
            state.specials.push(params);
            println!("Marked as special: {}", format_params(params));
        }
        Err(e) => eprintln!("Couldn't write to {}: {e}", specials_path::<A>().display()),
    }
}

fn append_special<A: Attractor>(params: [f64; 4]) -> io::Result<()> {
    fs::create_dir_all(output_dir::<A>())?;
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(specials_path::<A>())?;
    // Don't glue the new entry onto a last line that's missing its newline
    if file.metadata()?.len() > 0 {
        let mut last = [0u8];
        file.seek(SeekFrom::End(-1))?;
        file.read_exact(&mut last)?;
        if last[0] != b'\n' {
            writeln!(file)?;
        }
    }
    writeln!(file, "{}", format_params(params))
}

fn rgb(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | b as u32
}

/// A rectangle of the map window, `[left, right) x [top, bottom)`
#[derive(Clone, Copy)]
struct Rect {
    left: usize,
    top: usize,
    right: usize,
    bottom: usize,
}

impl Rect {
    fn contains(&self, x: f64, y: f64) -> bool {
        self.left as f64 <= x
            && x < self.right as f64
            && self.top as f64 <= y
            && y < self.bottom as f64
    }

    /// Set the pixel only if it's inside this rectangle
    fn set(&self, buf: &mut [u32], x: isize, y: isize, colour: u32) {
        if self.contains(x as f64, y as f64) {
            buf[y as usize * MAP_WIDTH + x as usize] = colour;
        }
    }

    fn param_to_px(&self, px: f64, py: f64) -> (isize, isize) {
        let (l, r, t, b) = (
            self.left as f64,
            self.right as f64,
            self.top as f64,
            self.bottom as f64,
        );
        let x = from_range_to_domain(px, -MAP_LIMIT, MAP_LIMIT, l, r) as isize;
        let y = from_range_to_domain(py, -MAP_LIMIT, MAP_LIMIT, t, b) as isize;
        (
            x.min(self.right as isize - 1),
            y.min(self.bottom as isize - 1),
        )
    }

    fn px_to_param(&self, x: f64, y: f64) -> (f64, f64) {
        (
            from_range_to_domain(
                x,
                self.left as f64,
                self.right as f64,
                -MAP_LIMIT,
                MAP_LIMIT,
            ),
            from_range_to_domain(
                y,
                self.top as f64,
                self.bottom as f64,
                -MAP_LIMIT,
                MAP_LIMIT,
            ),
        )
    }
}

/// Draw the four parameter plots, returning new parameters if the user is clicking on one.
fn update_map(
    map_buf: &mut [u32],
    params: &[f64; 4],
    specials: &[[f64; 4]],
    mouse_pos: Option<(f32, f32)>,
    mouse_down: bool,
    map_window: &mut Window,
) -> [Option<f64>; 4] {
    let color_axes = rgb(40, 40, 40);
    let color_specials = rgb(50, 50, 50);
    let color_specials_exact = rgb(0, 255, 0);
    let color_crosshairs = rgb(150, 150, 150);
    let color_crosshairs_mouse = rgb(150, 150, 150);

    // Which parameters each plot shows on its (x, y) axes:
    // 0:(a,b) 1:(c,b)
    // 2:(a,d) 3:(c,d)
    const PLOT_AXES: [(usize, usize); 4] = [(0, 1), (2, 1), (0, 3), (2, 3)];
    let (half_w, half_h) = (MAP_WIDTH / 2, MAP_HEIGHT / 2);
    let whole_window = Rect {
        left: 0,
        top: 0,
        right: MAP_WIDTH,
        bottom: MAP_HEIGHT,
    };

    map_buf.fill(0);
    let mut new_params = [None; 4];
    let mut mouse_in_a_plot = false;
    for (plt_idx, &(xi, yi)) in PLOT_AXES.iter().enumerate() {
        let left = (plt_idx % 2) * half_w;
        let top = (plt_idx / 2) * half_h;
        let plot = Rect {
            left,
            top,
            right: left + half_w,
            bottom: top + half_h,
        };

        let (zero_x, zero_y) = plot.param_to_px(0.0, 0.0);
        let x_border = MAP_WIDTH / 100;
        for x in (left + x_border)..(plot.right - x_border) {
            plot.set(map_buf, x as isize, zero_y, color_axes);
        }
        let y_border = MAP_HEIGHT / 100;
        for y in (top + y_border)..(plot.bottom - y_border) {
            plot.set(map_buf, zero_x, y as isize, color_axes);
        }

        // Specials are brighter the closer they are to the current parameters on the two axes
        // this plot doesn't show
        let (oxi, oyi) = PLOT_AXES[3 - plt_idx];
        for special in specials {
            let (mx, my) = plot.param_to_px(special[xi], special[yi]);
            let x_dist = (params[oxi] - special[oxi]).abs();
            let y_dist = (params[oyi] - special[oyi]).abs();
            let max_dist = 1.5;
            let colour = if x_dist < 0.1 && y_dist < 0.1 {
                color_specials_exact
            } else if x_dist < max_dist && y_dist < max_dist {
                let normalised_dist =
                    (x_dist * x_dist + y_dist * y_dist).sqrt() / (2_f64.sqrt() * max_dist);
                let brightness = 50 + (200.0 - 200.0 * normalised_dist) as u8;
                rgb(brightness, brightness, brightness)
            } else {
                color_specials
            };
            plot.set(map_buf, mx, my, colour);
        }

        // Cross-hairs on the current parameters, leaving the centre clear
        let (x, y) = plot.param_to_px(params[xi], params[yi]);
        for delta in (-30..30).step_by(3).filter(|d: &isize| d.abs() >= 10) {
            plot.set(map_buf, x + delta, y, color_crosshairs);
            plot.set(map_buf, x, y + delta, color_crosshairs);
        }

        if let Some((mousex, mousey)) = mouse_pos {
            let (mousex, mousey) = (mousex as f64, mousey as f64);
            if plot.contains(mousex, mousey) {
                mouse_in_a_plot = true;
                if mouse_down {
                    let (px, py) = plot.px_to_param(mousex, mousey);
                    new_params[xi] = Some(px);
                    new_params[yi] = Some(py);
                }
            }
        }
    }

    map_window.set_cursor_style(if mouse_in_a_plot {
        CursorStyle::Crosshair
    } else {
        CursorStyle::Arrow
    });
    if let Some((mousex, mousey)) = mouse_pos {
        let (mousex, mousey) = (mousex as isize, mousey as isize);
        for delta in -10..10 {
            whole_window.set(map_buf, mousex + delta, mousey, color_crosshairs_mouse);
            whole_window.set(map_buf, mousex, mousey + delta, color_crosshairs_mouse);
        }
    }
    new_params
}

fn from_range_to_domain(
    x: f64,
    lower_from: f64,
    upper_from: f64,
    lower_to: f64,
    upper_to: f64,
) -> f64 {
    (x.clamp(lower_from, upper_from) - lower_from) / (upper_from - lower_from)
        * (upper_to - lower_to)
        + lower_to
}
