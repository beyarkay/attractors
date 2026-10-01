use std::env;
use std::fs;
use std::path::Path;

use attractors::render::{save_png, Look};
use attractors::ui::{
    output_dir, paper_size, Buffers, Explorer, Input, HEIGHT, MAP_HEIGHT, MAP_WIDTH, WIDTH,
};
use attractors::*;
use minifb::{CursorStyle, Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};

const CACHE_DIR: &str = "cache";

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
        let dir = output_dir::<A>(Path::new(CACHE_DIR));
        match save_png(&attractor, &sampler, size, &view, &Look::default(), &dir) {
            Ok(path) => println!("Saved {}", path.display()),
            Err(e) => eprintln!("Failed to save `{line}`: {e}"),
        }
    }
}

/// The windows and pixel buffers, kept when switching between kinds of attractor
struct Windows {
    main: Window,
    map: Window,
    buffers: Buffers,
}

impl Windows {
    fn new() -> Self {
        let mut main = Window::new(
            "Strange Attractors",
            WIDTH,
            HEIGHT,
            WindowOptions::default(),
        )
        .unwrap_or_else(|e| panic!("{}", e));
        let mut map = Window::new(
            "Map: click and drag to choose parameters",
            MAP_WIDTH,
            MAP_HEIGHT,
            WindowOptions::default(),
        )
        .unwrap();
        map.set_position(0, 0);
        main.set_position(MAP_WIDTH as isize, 0);
        Windows {
            main,
            map,
            buffers: Buffers::default(),
        }
    }

    fn input(&self) -> Input {
        let main = &self.main;
        Input {
            pressed: main.get_keys_pressed(KeyRepeat::No),
            repeating: main.get_keys_pressed(KeyRepeat::Yes),
            shift: main.is_key_down(Key::LeftShift) || main.is_key_down(Key::RightShift),
            map_mouse: self.map.get_mouse_pos(MouseMode::Discard),
            map_mouse_down: self.map.get_mouse_down(MouseButton::Left),
        }
    }
}

/// Explore attractors of kind `A` until the window closes (returns `None`) or the user switches
/// to another kind (returns +1 or -1).
fn run_interactively<A: Attractor>(windows: &mut Windows) -> Option<isize> {
    let mut explorer = Explorer::<A>::new(Path::new(CACHE_DIR));
    println!("{}", explorer.help());
    println!("{}", explorer.description());
    let mut title = String::new();
    while windows.main.is_open() && !windows.main.is_key_down(Key::Escape) {
        let input = windows.input();
        let result = explorer.frame(&input, &mut windows.buffers);

        if windows.map.is_open() {
            windows.map.set_cursor_style(if result.map_crosshair {
                CursorStyle::Crosshair
            } else {
                CursorStyle::Arrow
            });
            windows
                .map
                .update_with_buffer(&windows.buffers.map, MAP_WIDTH, MAP_HEIGHT)
                .unwrap();
        }
        let new_title = explorer.title();
        if new_title != title {
            windows.main.set_title(&new_title);
            title = new_title;
        }
        windows
            .main
            .update_with_buffer(&windows.buffers.preview, WIDTH, HEIGHT)
            .unwrap();
        if result.switch_kind.is_some() {
            return result.switch_kind;
        }
    }
    None
}
