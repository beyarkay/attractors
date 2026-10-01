//! The interactive explorer, independent of any window: each frame takes the user's input and
//! fills pixel buffers, so it can be driven and checked without opening windows.

use std::fs::{self, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use minifb::Key;
use rand::Rng;

use crate::attractors::{format_params, lyapunov_exponent, parse_params, Attractor};
use crate::render::{blend_and_colour, save_png, Look, PALETTES};
use crate::sampler::{Histogram, Sampler, View};

pub const A0_600_DPI: (usize, usize) = (19866, 28087);
pub const A1_600_DPI: (usize, usize) = (14043, 19866);
pub const A2_600_DPI: (usize, usize) = (9933, 14043);
pub const A3_600_DPI: (usize, usize) = (7016, 9933);
pub const A4_600_DPI: (usize, usize) = (4960, 7016);
pub const A5_600_DPI: (usize, usize) = (3506, 4960);

pub const WIDTH: usize = 636;
pub const HEIGHT: usize = 900;
pub const MAP_WIDTH: usize = 600;
pub const MAP_HEIGHT: usize = 600;
const MIN_NUM_STEPS: usize = 500_000;
const MAX_PREVIEW_STEPS: usize = 10_000_000;
/// Number of parameters the nudge keys and the map act on at once
pub const PARAM_WINDOW: usize = 4;

pub fn paper_size(name: &str) -> Option<(usize, usize)> {
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

/// Where images and `special.txt` for this kind of attractor live
pub fn output_dir<A: Attractor>(cache_dir: &Path) -> PathBuf {
    cache_dir.join(A::NAME)
}

fn specials_path<A: Attractor>(cache_dir: &Path) -> PathBuf {
    output_dir::<A>(cache_dir).join("special.txt")
}

/// What the user did since the last frame
#[derive(Debug, Clone, Default)]
pub struct Input {
    /// Keys pressed since the last frame, without key repeat
    pub pressed: Vec<Key>,
    /// Keys pressed since the last frame, including repeats from keys held down
    pub repeating: Vec<Key>,
    /// Either Shift key is held
    pub shift: bool,
    /// Mouse position over the map window, if it's over it
    pub map_mouse: Option<(f32, f32)>,
    /// Left mouse button held over the map window
    pub map_mouse_down: bool,
}

impl Input {
    pub fn keys(keys: &[Key]) -> Self {
        Input {
            pressed: keys.to_vec(),
            repeating: keys.to_vec(),
            ..Input::default()
        }
    }

    pub fn shifted(mut self) -> Self {
        self.shift = true;
        self
    }

    pub fn click_map(x: f32, y: f32) -> Self {
        Input {
            map_mouse: Some((x, y)),
            map_mouse_down: true,
            ..Input::default()
        }
    }
}

/// The pixels of both windows, kept when switching between kinds of attractor so the preview
/// blends from one into the next
pub struct Buffers {
    pub preview: Vec<u32>,
    pub map: Vec<u32>,
    prev_densities: Vec<f64>,
}

impl Default for Buffers {
    fn default() -> Self {
        Buffers {
            preview: vec![0; WIDTH * HEIGHT],
            map: vec![0; MAP_WIDTH * MAP_HEIGHT],
            prev_densities: vec![0.0; WIDTH * HEIGHT],
        }
    }
}

/// What the window code should do after a frame
#[derive(Debug, Clone, PartialEq)]
pub struct FrameResult {
    /// Show a crosshair cursor over the map
    pub map_crosshair: bool,
    /// Move to another kind of attractor: +1 for the next, -1 for the previous
    pub switch_kind: Option<isize>,
}

pub struct Explorer<A> {
    attractor: A,
    sampler: Sampler,
    hist: Histogram,
    view: View,
    specials: Vec<Vec<f64>>,
    cache_dir: PathBuf,
    /// Index of the first parameter the nudge keys and map act on
    window_start: usize,
    /// Each frame is `noodle_factor * current + (1 - noodle_factor) * previous`, so values
    /// closer to 0 blend attractors into each other, reducing flashing
    noodle_factor: f64,
    /// How much each key press changes a parameter
    delta: f64,
    look: Look,
    switch_kind: Option<isize>,
    commands: Vec<Command<A>>,
}

impl<A: Attractor> Explorer<A> {
    /// Starts on a random special attractor from `cache_dir`, or the defaults if there are none
    pub fn new(cache_dir: &Path) -> Self {
        let specials = load_specials::<A>(cache_dir);
        let params = if specials.is_empty() {
            A::DEFAULT_PARAMS.to_vec()
        } else {
            specials[rand::thread_rng().gen_range(0..specials.len())].clone()
        };
        Self::with_params(cache_dir, &params)
    }

    pub fn with_params(cache_dir: &Path, params: &[f64]) -> Self {
        let mut explorer = Explorer {
            attractor: A::from_params(params),
            sampler: Sampler::new(A::START),
            hist: Histogram::new(WIDTH, HEIGHT),
            view: View::default(),
            specials: load_specials::<A>(cache_dir),
            cache_dir: cache_dir.to_path_buf(),
            window_start: 0,
            noodle_factor: 0.9,
            delta: 0.1,
            look: Look::default(),
            switch_kind: None,
            commands: commands(),
        };
        explorer.restart();
        explorer
    }

    pub fn attractor(&self) -> &A {
        &self.attractor
    }

    pub fn look(&self) -> Look {
        self.look
    }

    pub fn steps(&self) -> usize {
        self.hist.steps
    }

    /// Advance the simulation, apply the input, and draw both windows into `buffers`
    pub fn frame(&mut self, input: &Input, buffers: &mut Buffers) -> FrameResult {
        if self.hist.steps < MAX_PREVIEW_STEPS {
            self.sampler
                .step(&self.attractor, MIN_NUM_STEPS, &mut self.hist, &self.view);
        }
        blend_and_colour(
            &self.hist,
            &mut buffers.prev_densities,
            self.noodle_factor,
            &self.look,
            &mut buffers.preview,
        );

        let map = draw_map::<A>(
            &mut buffers.map,
            self.attractor.params(),
            self.window_start,
            &self.specials,
            input.map_mouse,
            input.map_mouse_down,
        );
        if !map.new_params.is_empty() {
            let mut attractor = self.attractor.clone();
            for (idx, value) in map.new_params {
                attractor = attractor.with_param(idx, value);
            }
            self.set_attractor(attractor);
        }

        // Take the commands out so actions can borrow the explorer mutably
        let commands = std::mem::take(&mut self.commands);
        for cmd in commands.iter().filter(|c| c.enabled) {
            let pressed = if cmd.repeat {
                &input.repeating
            } else {
                &input.pressed
            };
            if pressed.contains(&cmd.key) {
                (cmd.action)(self, input.shift);
            }
        }
        self.commands = commands;
        if input.pressed.contains(&Key::Slash) {
            println!("{}", self.help());
        }

        FrameResult {
            map_crosshair: map.mouse_in_a_plot,
            switch_kind: self.switch_kind.take(),
        }
    }

    /// The attractor's name, the parameters the keys currently control, the palette and step size
    pub fn title(&self) -> String {
        let end = (self.window_start + PARAM_WINDOW).min(A::PARAM_NAMES.len());
        let params: Vec<String> = (self.window_start..end)
            .map(|i| format!("{} {:+.3}", A::PARAM_NAMES[i], self.attractor.params()[i]))
            .collect();
        format!(
            "{}   {}   |   {}   |   step {}   |   ? for help",
            A::NAME,
            params.join("   "),
            self.look.palette().name,
            self.delta,
        )
    }

    pub fn help(&self) -> String {
        let mut help = String::from("\nKeys (with the attractor window focused):\n");
        for command in self.commands.iter().filter(|c| c.enabled) {
            let key = match command.key {
                Key::Semicolon => ";".to_string(),
                Key::LeftBracket => "[".to_string(),
                Key::RightBracket => "]".to_string(),
                key => format!("{key:?}"),
            };
            help.push_str(&format!("  {key:>3}  {}\n", command.description));
        }
        help.push_str("    ?  Show this help again\n  Esc  Quit\n");
        help
    }

    /// The formula and current parameters, printed when this kind of attractor is selected
    pub fn description(&self) -> String {
        format!(
            "=== {} ({} parameters) ===\n{}\n{}",
            A::NAME,
            A::PARAM_NAMES.len(),
            A::FORMULA,
            format_params(A::PARAM_NAMES, self.attractor.params())
        )
    }

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

    fn mark_special(&mut self) {
        let params = self.attractor.params().to_vec();
        let line = format_params(A::PARAM_NAMES, &params);
        if self.specials.contains(&params) {
            println!("Already marked as special: {line}");
            return;
        }
        match append_special::<A>(&self.cache_dir, &line) {
            Ok(()) => {
                self.specials.push(params);
                println!("Marked as special: {line}");
            }
            Err(e) => eprintln!(
                "Couldn't write to {}: {e}",
                specials_path::<A>(&self.cache_dir).display()
            ),
        }
    }
}

struct Command<A> {
    key: Key,
    /// Whether holding the key keeps triggering the action
    repeat: bool,
    description: &'static str,
    enabled: bool,
    /// Called with whether either Shift key is held
    action: fn(&mut Explorer<A>, bool),
}

fn sign(shift: bool) -> f64 {
    if shift {
        -1.0
    } else {
        1.0
    }
}

fn nudge<A: Attractor, const OFFSET: usize, const UP: bool>(s: &mut Explorer<A>, _: bool) {
    s.nudge(OFFSET, if UP { 1.0 } else { -1.0 });
}

fn nudge_command<A>(
    key: Key,
    action: fn(&mut Explorer<A>, bool),
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
        nudge_command(Key::J, nudge::<A, 0, false>, "Decrease the 1st parameter in the window by the step size"),
        nudge_command(Key::K, nudge::<A, 1, false>, "Decrease the 2nd parameter in the window by the step size"),
        nudge_command(Key::L, nudge::<A, 2, false>, "Decrease the 3rd parameter in the window by the step size"),
        nudge_command(Key::Semicolon, nudge::<A, 3, false>, "Decrease the 4th parameter in the window by the step size"),
        nudge_command(Key::U, nudge::<A, 0, true>, "Increase the 1st parameter in the window by the step size"),
        nudge_command(Key::I, nudge::<A, 1, true>, "Increase the 2nd parameter in the window by the step size"),
        nudge_command(Key::O, nudge::<A, 2, true>, "Increase the 3rd parameter in the window by the step size"),
        nudge_command(Key::P, nudge::<A, 3, true>, "Increase the 4th parameter in the window by the step size"),
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
                let dir = output_dir::<A>(&s.cache_dir);
                match save_png(&s.attractor, &s.sampler, size, &s.view, &s.look, &dir) {
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
            action: |s, _| s.mark_special(),
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
            description: "Multiply the step size by 10 (Shift: divide by 10)",
            action: |s, shift| {
                s.delta *= 10.0_f64.powf(sign(shift));
                println!("step size: {}", s.delta);
            },
        },
        Command {
            key: Key::D,
            repeat: true,
            enabled: true,
            description: "Increase contrast, so faint areas get darker (Shift: decrease)",
            action: |s, shift| {
                s.look.contrast = (s.look.contrast * 1.05_f64.powf(sign(shift))).clamp(0.2, 5.0);
                println!("contrast: {:.2}", s.look.contrast);
            },
        },
        Command {
            key: Key::C,
            repeat: false,
            enabled: true,
            description: "Next colour palette (Shift: previous)",
            action: |s, shift| {
                let n = PALETTES.len();
                s.look.palette = (s.look.palette + if shift { n - 1 } else { 1 }) % n;
                println!("palette: {}", s.look.palette().name);
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

fn load_specials<A: Attractor>(cache_dir: &Path) -> Vec<Vec<f64>> {
    let path = specials_path::<A>(cache_dir);
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

fn append_special<A: Attractor>(cache_dir: &Path, line: &str) -> io::Result<()> {
    fs::create_dir_all(output_dir::<A>(cache_dir))?;
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(specials_path::<A>(cache_dir))?;
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

pub fn rgb(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | b as u32
}

pub const MAP_BACKGROUND: u32 = 0x0d0f18;
pub const MAP_CROSSHAIRS: u32 = 0xf0f0fa;

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

struct MapResult {
    /// `(parameter index, new value)` for each parameter the user is clicking on
    new_params: Vec<(usize, f64)>,
    mouse_in_a_plot: bool,
}

/// Which parameters of the window each quadrant shows on its (x, y) axes:
/// 0:(1st,2nd) 1:(3rd,2nd)
/// 2:(1st,4th) 3:(3rd,4th)
const PLOT_AXES: [(usize, usize); 4] = [(0, 1), (2, 1), (0, 3), (2, 3)];

fn plot_rect(plt_idx: usize) -> Rect {
    let (half_w, half_h) = (MAP_WIDTH / 2, MAP_HEIGHT / 2);
    let left = (plt_idx % 2) * half_w;
    let top = (plt_idx / 2) * half_h;
    Rect {
        left,
        top,
        right: left + half_w,
        bottom: top + half_h,
    }
}

/// Draw up to four plots of parameter pairs from the window starting at `window_start`
fn draw_map<A: Attractor>(
    map_buf: &mut [u32],
    params: &[f64],
    window_start: usize,
    specials: &[Vec<f64>],
    mouse_pos: Option<(f32, f32)>,
    mouse_down: bool,
) -> MapResult {
    let color_dividers = rgb(60, 66, 88);
    let color_axes = rgb(34, 39, 56);
    let color_specials = [62, 70, 98];
    let color_specials_near = [255, 184, 92];
    let color_specials_exact = rgb(110, 255, 170);
    let color_crosshairs_mouse = rgb(120, 130, 155);

    let range = A::PARAM_RANGE;
    let (half_w, half_h) = (MAP_WIDTH / 2, MAP_HEIGHT / 2);
    let whole_window = Rect {
        left: 0,
        top: 0,
        right: MAP_WIDTH,
        bottom: MAP_HEIGHT,
    };

    map_buf.fill(MAP_BACKGROUND);
    for i in 0..MAP_WIDTH.min(MAP_HEIGHT) {
        whole_window.set(map_buf, half_w as isize, i as isize, color_dividers);
        whole_window.set(map_buf, i as isize, half_h as isize, color_dividers);
    }
    let mut new_params = Vec::new();
    let mut mouse_in_a_plot = false;
    for (plt_idx, &(x_offset, y_offset)) in PLOT_AXES.iter().enumerate() {
        let (xi, yi) = (window_start + x_offset, window_start + y_offset);
        if xi >= params.len() || yi >= params.len() {
            continue;
        }
        let plot = plot_rect(plt_idx);

        let (zero_x, zero_y) = plot.param_to_px(0.0, 0.0, range);
        let x_border = MAP_WIDTH / 100;
        for x in (plot.left + x_border)..(plot.right - x_border) {
            plot.set(map_buf, x as isize, zero_y, color_axes);
        }
        let y_border = MAP_HEIGHT / 100;
        for y in (plot.top + y_border)..(plot.bottom - y_border) {
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
                let near = 1.0 - dist / max_dist;
                let [r, g, b] = [0, 1, 2].map(|i| {
                    (color_specials[i] as f64
                        + near * (color_specials_near[i] as f64 - color_specials[i] as f64))
                        as u8
                });
                rgb(r, g, b)
            } else {
                let [r, g, b] = color_specials;
                rgb(r, g, b)
            };
            plot.set(map_buf, mx, my, colour);
        }

        // Cross-hairs on the current parameters, leaving the centre clear
        let (x, y) = plot.param_to_px(params[xi], params[yi], range);
        for delta in (-30..30).step_by(3).filter(|d: &isize| d.abs() >= 10) {
            plot.set(map_buf, x + delta, y, MAP_CROSSHAIRS);
            plot.set(map_buf, x, y + delta, MAP_CROSSHAIRS);
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

    if let Some((mousex, mousey)) = mouse_pos {
        let (mousex, mousey) = (mousex as isize, mousey as isize);
        for delta in -10..10 {
            whole_window.set(map_buf, mousex + delta, mousey, color_crosshairs_mouse);
            whole_window.set(map_buf, mousex, mousey + delta, color_crosshairs_mouse);
        }
    }
    MapResult {
        new_params,
        mouse_in_a_plot,
    }
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
