use std::env;
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
const WIDTH: usize = ISO_PAPER_FORMAT.0;
const HEIGHT: usize = ISO_PAPER_FORMAT.1;
const MIN_NUM_STEPS: usize = 500_000;
const MAX_PREVIEW_STEPS: usize = 10_000_000;
const DEFAULT_POWER: f64 = 0.15;
/// Number of parameters the nudge keys and the map act on at once
const PARAM_WINDOW: usize = 4;

/// Everything the program can do with one kind of attractor
struct Kind {
    name: &'static str,
    explore: fn(&mut Windows) -> Option<isize>,
    render_file: fn(&str, (usize, usize)),
}

fn kind<A: Attractor>() -> Kind {
    Kind {
        name: A::NAME,
        explore: run_interactively::<A>,
        render_file: run_headless::<A>,
    }
}

/// Every attractor the program knows about. Add new ones here.
fn kinds() -> Vec<Kind> {
    vec![
        kind::<CliffordAttractor>(),
        kind::<DeJongAttractor>(),
        kind::<KingsDreamAttractor>(),
        kind::<HenonAttractor>(),
        kind::<TinkerbellAttractor>(),
        kind::<SprottQuadraticAttractor>(),
    ]
}

/// Where images and `special.txt` for this kind of attractor live
fn output_dir<A: Attractor>() -> PathBuf {
    Path::new("cache").join(A::NAME)
}

fn specials_path<A: Attractor>() -> PathBuf {
    output_dir::<A>().join("special.txt")
}

fn main() {
    let mut args: Vec<String> = env::args().skip(1).collect();
    let kinds = kinds();
    let mut name = kinds[0].name.to_string();
    if let Some(i) = args.iter().position(|a| a == "--attractor") {
        args.remove(i);
        if i < args.len() {
            name = args.remove(i);
        }
    }
    let usage = format!(
        "Usage: attractors [--attractor NAME] [<file of parameter lines, e.g. `a=..,b=..`> [A0|A1|A2|A3|A4|A5]]\n\
         Attractors: {}",
        kinds.iter().map(|k| k.name).collect::<Vec<_>>().join(", ")
    );
    let Some(mut idx) = kinds.iter().position(|k| k.name == name) else {
        println!("Unknown attractor `{name}`\n{usage}");
        return;
    };
    match args.as_slice() {
        [] => {
            let mut windows = Windows::new();
            while let Some(step) = (kinds[idx].explore)(&mut windows) {
                idx = (idx as isize + step).rem_euclid(kinds.len() as isize) as usize;
            }
        }
        [file] | [file, _] => {
            let paper = args.get(1).map_or("A0", String::as_str);
            match paper_size(paper) {
                Some(size) => (kinds[idx].render_file)(file, size),
                None => println!("Unknown paper size `{paper}`\n{usage}"),
            }
        }
        _ => println!("{usage}"),
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

fn run_headless<A: Attractor>(filename: &str, size: (usize, usize)) {
    let contents = fs::read_to_string(filename).expect("Something went wrong reading the file");
    let lines: Vec<_> = contents
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty() && !l.starts_with('#'))
        .collect();
    println!("About to create {} attractors and save them", lines.len());

    for (i, line) in lines {
        let Some(params) = parse_params(line, A::PARAM_NAMES) else {
            eprintln!(
                "{filename}:{}: couldn't parse `{line}` as {} parameters ({})",
                i + 1,
                A::NAME,
                A::PARAM_NAMES.join(", ")
            );
            continue;
        };
        let attractor = A::from_params(&params);
        let sampler = Sampler::new(A::START);
        let view = View::default();
        let dir = output_dir::<A>();
        match save_png(&attractor, &sampler, size, &view, DEFAULT_POWER, &dir) {
            Ok(path) => println!("Saved {}", path.display()),
            Err(e) => eprintln!("Failed to save `{line}`: {e}"),
        }
    }
}

/// Random parameters in `A::PARAM_RANGE`, retried until the attractor is chaotic
fn random_chaotic_params<A: Attractor>() -> Vec<f64> {
    let mut rng = rand::thread_rng();
    let (lo, hi) = A::PARAM_RANGE;
    for tries in 1..=10_000 {
        let params: Vec<f64> = (0..A::PARAM_NAMES.len())
            .map(|_| rng.gen_range(lo..hi))
            .collect();
        if lyapunov_exponent(&A::from_params(&params), 2_000) > 0.005 {
            println!("Found chaotic parameters after {tries} tries");
            return params;
        }
    }
    println!("No chaotic parameters found, using the defaults");
    A::DEFAULT_PARAMS.to_vec()
}

/// The windows and pixel buffers, kept when switching between kinds of attractor
struct Windows {
    main: Window,
    map: Window,
    buffer: Vec<u32>,
    prev_densities: Vec<f64>,
    map_buf: Vec<u32>,
}

impl Windows {
    fn new() -> Self {
        let mut main = Window::new(
            "Strange Attractors (hold esc to exit)",
            WIDTH,
            HEIGHT,
            WindowOptions::default(),
        )
        .unwrap_or_else(|e| panic!("{}", e));
        let mut map = Window::new("Map", MAP_WIDTH, MAP_HEIGHT, WindowOptions::default()).unwrap();
        map.set_position(0, 0);
        main.set_position(MAP_WIDTH as isize, 0);
        Windows {
            main,
            map,
            buffer: vec![0; WIDTH * HEIGHT],
            prev_densities: vec![0.0; WIDTH * HEIGHT],
            map_buf: vec![0; MAP_WIDTH * MAP_HEIGHT],
        }
    }
}

struct State<A> {
    attractor: A,
    sampler: Sampler,
    hist: Histogram,
    view: View,
    specials: Vec<Vec<f64>>,
    /// Index of the first parameter the nudge keys and map act on
    window_start: usize,
    /// Each frame is `noodle_factor * current + (1 - noodle_factor) * previous`, so values
    /// closer to 0 blend attractors into each other, reducing flashing
    noodle_factor: f64,
    /// How much each key press changes a parameter
    delta: f64,
    /// Densities are raised to this power before colouring
    power: f64,
    /// Set to move to another kind of attractor: +1 for the next, -1 for the previous
    switch_kind: Option<isize>,
}

impl<A: Attractor> State<A> {
    fn restart(&mut self) {
        self.hist.clear();
        self.sampler
            .step(&self.attractor, MIN_NUM_STEPS, &mut self.hist, &self.view);
    }

    fn set_attractor(&mut self, attractor: A) {
        self.attractor = attractor;
        println!("{}", format_params(A::PARAM_NAMES, self.attractor.params()));
        self.sampler.reset();
        self.restart();
    }

    fn nudge(&mut self, offset: usize, sign: f64) {
        let idx = self.window_start + offset;
        if idx < A::PARAM_NAMES.len() {
            let value = self.attractor.params()[idx] + sign * self.delta;
            self.set_attractor(self.attractor.with_param(idx, value));
        }
    }

    fn set_view(&mut self, view: View) {
        self.view = view;
        self.restart();
    }

    fn shift_window(&mut self, by: isize) {
        let last_start = A::PARAM_NAMES.len().saturating_sub(PARAM_WINDOW);
        self.window_start =
            (self.window_start as isize + by).clamp(0, last_start as isize) as usize;
        let end = (self.window_start + PARAM_WINDOW).min(A::PARAM_NAMES.len());
        println!(
            "Keys and map now control {}",
            A::PARAM_NAMES[self.window_start..end].join(", ")
        );
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

fn nudge<A: Attractor, const OFFSET: usize, const UP: bool>(s: &mut State<A>, _: bool) {
    s.nudge(OFFSET, if UP { 1.0 } else { -1.0 });
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

fn commands<A: Attractor>() -> Vec<Command<A>> {
    vec![
        nudge_command(Key::J, nudge::<A, 0, false>, "1st parameter in the window -= delta"),
        nudge_command(Key::K, nudge::<A, 1, false>, "2nd parameter in the window -= delta"),
        nudge_command(Key::L, nudge::<A, 2, false>, "3rd parameter in the window -= delta"),
        nudge_command(Key::Semicolon, nudge::<A, 3, false>, "4th parameter in the window -= delta"),
        nudge_command(Key::U, nudge::<A, 0, true>, "1st parameter in the window += delta"),
        nudge_command(Key::I, nudge::<A, 1, true>, "2nd parameter in the window += delta"),
        nudge_command(Key::O, nudge::<A, 2, true>, "3rd parameter in the window += delta"),
        nudge_command(Key::P, nudge::<A, 3, true>, "4th parameter in the window += delta"),
        Command {
            key: Key::RightBracket,
            repeat: false,
            enabled: true,
            description: "Slide the window of parameters the keys and map control to the right",
            action: |s, _| s.shift_window(1),
        },
        Command {
            key: Key::LeftBracket,
            repeat: false,
            enabled: true,
            description: "Slide the window of parameters the keys and map control to the left",
            action: |s, _| s.shift_window(-1),
        },
        Command {
            key: Key::A,
            repeat: false,
            enabled: true,
            description: "Switch to the next kind of attractor (Shift: previous)",
            action: |s, shift| s.switch_kind = Some(sign(shift) as isize),
        },
        Command {
            key: Key::R,
            repeat: false,
            enabled: true,
            description: "Jump to a random special attractor (Shift: random chaotic parameters)",
            action: |s, shift| {
                let params = if shift || s.specials.is_empty() {
                    random_chaotic_params::<A>()
                } else {
                    s.specials[rand::thread_rng().gen_range(0..s.specials.len())].clone()
                };
                s.view = View::default();
                s.set_attractor(A::from_params(&params));
            },
        },
        Command {
            key: Key::S,
            repeat: false,
            enabled: true,
            description: "Save the attractor as an A3 png (Shift: A0)",
            action: |s, shift| {
                let size = if shift { A0_600_DPI } else { A3_600_DPI };
                let dir = output_dir::<A>();
                match save_png(&s.attractor, &s.sampler, size, &s.view, s.power, &dir) {
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

/// Explore attractors of kind `A` until the window closes (returns `None`) or the user switches
/// to another kind (returns +1 or -1).
fn run_interactively<A: Attractor>(windows: &mut Windows) -> Option<isize> {
    let specials = load_specials::<A>();
    let params = if specials.is_empty() {
        A::DEFAULT_PARAMS.to_vec()
    } else {
        specials[rand::thread_rng().gen_range(0..specials.len())].clone()
    };
    let mut state = State {
        attractor: A::from_params(&params),
        sampler: Sampler::new(A::START),
        hist: Histogram::new(WIDTH, HEIGHT),
        view: View::default(),
        specials,
        window_start: 0,
        noodle_factor: 0.9,
        delta: 0.1,
        power: DEFAULT_POWER,
        switch_kind: None,
    };

    let commands = commands();
    println!("=== List of Commands ===");
    for command in commands.iter().filter(|c| c.enabled) {
        println!("`{:?}` => {}", command.key, command.description);
    }
    println!(
        "=== {} ({} parameters) ===\n{}",
        A::NAME,
        A::PARAM_NAMES.len(),
        A::FORMULA
    );
    println!(
        "{}",
        format_params(A::PARAM_NAMES, state.attractor.params())
    );
    windows.main.set_title(&format!(
        "Strange Attractors: {} (hold esc to exit)",
        A::NAME
    ));

    state.restart();
    while windows.main.is_open() && !windows.main.is_key_down(Key::Escape) {
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
            &mut windows.prev_densities,
            state.noodle_factor,
            state.power,
            &mut windows.buffer,
        );

        if windows.map.is_open() {
            let new_params = update_map::<A>(
                &mut windows.map_buf,
                state.attractor.params(),
                state.window_start,
                &state.specials,
                windows.map.get_mouse_pos(MouseMode::Discard),
                windows.map.get_mouse_down(MouseButton::Left),
                &mut windows.map,
            );
            windows
                .map
                .update_with_buffer(&windows.map_buf, MAP_WIDTH, MAP_HEIGHT)
                .unwrap();
            if !new_params.is_empty() {
                let mut attractor = state.attractor.clone();
                for (idx, value) in new_params {
                    attractor = attractor.with_param(idx, value);
                }
                state.set_attractor(attractor);
            }
        }

        let window = &windows.main;
        let shift = window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift);
        let repeating = window.get_keys_pressed(KeyRepeat::Yes);
        let once = window.get_keys_pressed(KeyRepeat::No);
        for cmd in commands.iter().filter(|c| c.enabled) {
            let pressed = if cmd.repeat { &repeating } else { &once };
            if pressed.contains(&cmd.key) {
                (cmd.action)(&mut state, shift);
            }
        }
        windows
            .main
            .update_with_buffer(&windows.buffer, WIDTH, HEIGHT)
            .unwrap();
        if state.switch_kind.is_some() {
            return state.switch_kind;
        }
    }
    None
}

fn load_specials<A: Attractor>() -> Vec<Vec<f64>> {
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
            let params = parse_params(line, A::PARAM_NAMES);
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
    let params = state.attractor.params().to_vec();
    let line = format_params(A::PARAM_NAMES, &params);
    if state.specials.contains(&params) {
        println!("Already marked as special: {line}");
        return;
    }
    match append_special::<A>(&line) {
        Ok(()) => {
            state.specials.push(params);
            println!("Marked as special: {line}");
        }
        Err(e) => eprintln!("Couldn't write to {}: {e}", specials_path::<A>().display()),
    }
}

fn append_special<A: Attractor>(line: &str) -> io::Result<()> {
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
    writeln!(file, "{line}")
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

    fn param_to_px(&self, px: f64, py: f64, range: (f64, f64)) -> (isize, isize) {
        let (lo, hi) = range;
        let x = from_range_to_domain(px, lo, hi, self.left as f64, self.right as f64) as isize;
        let y = from_range_to_domain(py, lo, hi, self.top as f64, self.bottom as f64) as isize;
        (
            x.min(self.right as isize - 1),
            y.min(self.bottom as isize - 1),
        )
    }

    fn px_to_param(&self, x: f64, y: f64, range: (f64, f64)) -> (f64, f64) {
        let (lo, hi) = range;
        (
            from_range_to_domain(x, self.left as f64, self.right as f64, lo, hi),
            from_range_to_domain(y, self.top as f64, self.bottom as f64, lo, hi),
        )
    }
}

/// Draw up to four plots of parameter pairs from the window starting at `window_start`,
/// returning `(parameter index, new value)` for each parameter the user is clicking on.
fn update_map<A: Attractor>(
    map_buf: &mut [u32],
    params: &[f64],
    window_start: usize,
    specials: &[Vec<f64>],
    mouse_pos: Option<(f32, f32)>,
    mouse_down: bool,
    map_window: &mut Window,
) -> Vec<(usize, f64)> {
    let color_axes = rgb(40, 40, 40);
    let color_specials = rgb(50, 50, 50);
    let color_specials_exact = rgb(0, 255, 0);
    let color_crosshairs = rgb(150, 150, 150);
    let color_crosshairs_mouse = rgb(150, 150, 150);

    // Which parameters of the window each quadrant shows on its (x, y) axes:
    // 0:(1st,2nd) 1:(3rd,2nd)
    // 2:(1st,4th) 3:(3rd,4th)
    const PLOT_AXES: [(usize, usize); 4] = [(0, 1), (2, 1), (0, 3), (2, 3)];
    let range = A::PARAM_RANGE;
    let (half_w, half_h) = (MAP_WIDTH / 2, MAP_HEIGHT / 2);
    let whole_window = Rect {
        left: 0,
        top: 0,
        right: MAP_WIDTH,
        bottom: MAP_HEIGHT,
    };

    map_buf.fill(0);
    let mut new_params = Vec::new();
    let mut mouse_in_a_plot = false;
    for (plt_idx, &(x_offset, y_offset)) in PLOT_AXES.iter().enumerate() {
        let (xi, yi) = (window_start + x_offset, window_start + y_offset);
        if xi >= params.len() || yi >= params.len() {
            continue;
        }
        let left = (plt_idx % 2) * half_w;
        let top = (plt_idx / 2) * half_h;
        let plot = Rect {
            left,
            top,
            right: left + half_w,
            bottom: top + half_h,
        };

        let (zero_x, zero_y) = plot.param_to_px(0.0, 0.0, range);
        let x_border = MAP_WIDTH / 100;
        for x in (left + x_border)..(plot.right - x_border) {
            plot.set(map_buf, x as isize, zero_y, color_axes);
        }
        let y_border = MAP_HEIGHT / 100;
        for y in (top + y_border)..(plot.bottom - y_border) {
            plot.set(map_buf, zero_x, y as isize, color_axes);
        }

        // Specials are brighter the closer they are to the current parameters on every axis
        // this plot doesn't show, measured relative to the parameter range
        let width = range.1 - range.0;
        let others = (params.len() - 2).max(1) as f64;
        for special in specials {
            let (mx, my) = plot.param_to_px(special[xi], special[yi], range);
            let squared: f64 = (0..params.len())
                .filter(|&k| k != xi && k != yi)
                .map(|k| (params[k] - special[k]).powi(2))
                .sum();
            let dist = squared.sqrt() / (width * others.sqrt());
            let max_dist = 0.15;
            let colour = if dist < 0.01 {
                color_specials_exact
            } else if dist < max_dist {
                let brightness = 50 + (200.0 - 200.0 * dist / max_dist) as u8;
                rgb(brightness, brightness, brightness)
            } else {
                color_specials
            };
            plot.set(map_buf, mx, my, colour);
        }

        // Cross-hairs on the current parameters, leaving the centre clear
        let (x, y) = plot.param_to_px(params[xi], params[yi], range);
        for delta in (-30..30).step_by(3).filter(|d: &isize| d.abs() >= 10) {
            plot.set(map_buf, x + delta, y, color_crosshairs);
            plot.set(map_buf, x, y + delta, color_crosshairs);
        }

        if let Some((mousex, mousey)) = mouse_pos {
            let (mousex, mousey) = (mousex as f64, mousey as f64);
            if plot.contains(mousex, mousey) {
                mouse_in_a_plot = true;
                if mouse_down {
                    let (px, py) = plot.px_to_param(mousex, mousey, range);
                    new_params.push((xi, px));
                    new_params.push((yi, py));
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
