use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use attractors::render::{save_png, Look};
use attractors::ui::{
    output_dir, Buffers, Explorer, Input, A0_600_DPI, A1_600_DPI, A2_600_DPI, A3_600_DPI,
    A4_600_DPI, A5_600_DPI, HEIGHT, MAP_HEIGHT, MAP_WIDTH, WIDTH,
};
use attractors::*;
use clap::builder::PossibleValuesParser;
use clap::{Parser, ValueEnum};
use minifb::{CursorStyle, Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};

const CACHE_DIR: &str = "cache";

/// Everything the program can do with one kind of attractor
struct Kind {
    name: &'static str,
    explore: fn(&mut Windows) -> Option<isize>,
    render_file: fn(&Path, (usize, usize)) -> io::Result<()>,
}

fn kind<A: Attractor>() -> Kind {
    Kind {
        name: A::NAME,
        explore: run_interactively::<A>,
        render_file: run_headless::<A>,
    }
}

macro_rules! kinds_of {
    ($($t:ty),*) => {
        vec![$(kind::<$t>()),*]
    };
}

/// Every attractor the program knows about, from `attractors::all_attractors!`
fn kinds() -> Vec<Kind> {
    attractors::all_attractors!(kinds_of)
}

/// Explore strange attractors in a window, or render a file of parameters to PNGs
#[derive(Parser, Debug)]
#[command(version)]
struct Cli {
    /// Kind of attractor (`a` / `A` switch kinds while exploring)
    #[arg(short, long, default_value = "clifford", value_parser = PossibleValuesParser::new(attractor_names()))]
    attractor: String,

    /// File of parameter lines like `a=1.2,b=-0.5,c=1,d=0.7` to render to `cache/<attractor>/`
    /// without opening a window
    file: Option<PathBuf>,

    /// Paper size to render at, 600 dots per inch
    #[arg(value_enum, ignore_case = true, default_value_t = Paper::A0, requires = "file")]
    paper: Paper,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq)]
enum Paper {
    A0,
    A1,
    A2,
    A3,
    A4,
    A5,
}

impl Paper {
    fn pixels(self) -> (usize, usize) {
        match self {
            Paper::A0 => A0_600_DPI,
            Paper::A1 => A1_600_DPI,
            Paper::A2 => A2_600_DPI,
            Paper::A3 => A3_600_DPI,
            Paper::A4 => A4_600_DPI,
            Paper::A5 => A5_600_DPI,
        }
    }
}

fn attractor_names() -> Vec<&'static str> {
    kinds().iter().map(|k| k.name).collect()
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let kinds = kinds();
    let mut idx = kinds
        .iter()
        .position(|k| k.name == cli.attractor)
        .expect("clap only accepts known attractor names");
    match &cli.file {
        None => {
            let mut windows = Windows::new();
            while let Some(step) = (kinds[idx].explore)(&mut windows) {
                idx = (idx as isize + step).rem_euclid(kinds.len() as isize) as usize;
            }
        }
        Some(file) => {
            if let Err(e) = (kinds[idx].render_file)(file, cli.paper.pixels()) {
                eprintln!("Couldn't read {}: {e}", file.display());
                return ExitCode::FAILURE;
            }
        }
    }
    ExitCode::SUCCESS
}

fn run_headless<A: Attractor>(filename: &Path, size: (usize, usize)) -> io::Result<()> {
    let contents = fs::read_to_string(filename)?;
    let lines: Vec<_> = contents
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty() && !l.starts_with('#'))
        .collect();
    println!("About to create {} attractors and save them", lines.len());

    for (i, line) in lines {
        let Some(params) = parse_params(line, A::PARAM_NAMES) else {
            eprintln!(
                "{}:{}: couldn't parse `{line}` as {} parameters ({})",
                filename.display(),
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
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::error::ErrorKind;
    use clap::CommandFactory;

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(std::iter::once("attractors").chain(args.iter().copied()))
    }

    #[test]
    fn clap_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn no_arguments_explores_clifford() {
        let cli = parse(&[]).unwrap();
        assert_eq!(cli.attractor, "clifford");
        assert_eq!(cli.file, None);
    }

    #[test]
    fn help_and_version_are_not_files() {
        assert_eq!(
            parse(&["--help"]).unwrap_err().kind(),
            ErrorKind::DisplayHelp
        );
        assert_eq!(parse(&["-h"]).unwrap_err().kind(), ErrorKind::DisplayHelp);
        assert_eq!(
            parse(&["--version"]).unwrap_err().kind(),
            ErrorKind::DisplayVersion
        );
    }

    #[test]
    fn help_lists_every_attractor() {
        let help = Cli::command().render_long_help().to_string();
        for name in attractor_names() {
            assert!(help.contains(name), "{name} missing from:\n{help}");
        }
    }

    #[test]
    fn rejects_unknown_attractors_and_options() {
        assert_eq!(
            parse(&["-a", "nope"]).unwrap_err().kind(),
            ErrorKind::InvalidValue
        );
        assert_eq!(
            parse(&["--frobnicate"]).unwrap_err().kind(),
            ErrorKind::UnknownArgument
        );
    }

    #[test]
    fn file_and_paper_size() {
        let cli = parse(&["--attractor", "henon", "params.txt", "a3"]).unwrap();
        assert_eq!(cli.attractor, "henon");
        assert_eq!(cli.file, Some(PathBuf::from("params.txt")));
        assert_eq!(cli.paper, Paper::A3);
        assert_eq!(parse(&["params.txt", "A5"]).unwrap().paper, Paper::A5);
        assert_eq!(parse(&["params.txt"]).unwrap().paper, Paper::A0);
        assert_eq!(
            parse(&["params.txt", "B4"]).unwrap_err().kind(),
            ErrorKind::InvalidValue
        );
    }

    #[test]
    fn missing_file_is_an_error_not_a_crash() {
        let missing = Path::new("/definitely/not/here.txt");
        let err = run_headless::<CliffordAttractor>(missing, A5_600_DPI).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }
}
