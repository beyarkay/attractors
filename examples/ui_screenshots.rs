//! Drive the explorer with scripted input, without opening windows, and save what both windows
//! would show as PNGs plus an HTML page to look through.
//!
//! Usage: cargo run --release --example ui_screenshots -- <out dir>

use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::Path;

use attractors::ui::{Buffers, Explorer, Input, HEIGHT, MAP_HEIGHT, MAP_WIDTH, WIDTH};
use attractors::*;
use minifb::Key;

/// Frames for the preview to reach its full number of points
const SETTLE: usize = 20;

fn write_rgb(pixels: &[u32], width: usize, height: usize, path: &Path) {
    let mut encoder = png::Encoder::new(
        BufWriter::new(File::create(path).unwrap()),
        width as u32,
        height as u32,
    );
    encoder.set_color(png::ColorType::Rgb);
    let rgb: Vec<u8> = pixels
        .iter()
        .flat_map(|c| [(c >> 16) as u8, (c >> 8) as u8, *c as u8])
        .collect();
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&rgb)
        .unwrap();
}

struct Shot {
    name: &'static str,
    /// Inputs applied one per frame before letting the preview settle
    inputs: Vec<Input>,
    /// Input held during the frames after, e.g. the mouse hovering over the map
    hold: Input,
}

fn shots(many_params: bool) -> Vec<Shot> {
    let mut shots = vec![
        Shot {
            name: "start",
            inputs: vec![],
            hold: Input::default(),
        },
        Shot {
            name: "after pressing u and k",
            inputs: vec![Input::keys(&[Key::U]), Input::keys(&[Key::K])],
            hold: Input::default(),
        },
        Shot {
            name: "hovering over the map",
            inputs: vec![],
            hold: Input {
                map_mouse: Some((420.0, 380.0)),
                ..Input::default()
            },
        },
        Shot {
            name: "after pressing c twice",
            inputs: vec![Input::keys(&[Key::C]), Input::keys(&[Key::C])],
            hold: Input::default(),
        },
    ];
    if many_params {
        shots.push(Shot {
            name: "after pressing ] four times",
            inputs: vec![Input::keys(&[Key::RightBracket]); 4],
            hold: Input::default(),
        });
    }
    shots
}

fn screenshots<A: Attractor>(out_dir: &Path, html: &mut String) {
    let cache = Path::new("cache");
    writeln!(html, "<h2>{}</h2>", A::NAME).unwrap();
    for (i, shot) in shots(A::PARAM_NAMES.len() > 4).iter().enumerate() {
        let mut explorer = Explorer::<A>::with_params(cache, A::DEFAULT_PARAMS);
        let mut buffers = Buffers::default();
        for input in &shot.inputs {
            explorer.frame(input, &mut buffers);
        }
        for _ in 0..SETTLE {
            explorer.frame(&shot.hold, &mut buffers);
        }
        let stem = format!("{}_{i}", A::NAME);
        write_rgb(
            &buffers.preview,
            WIDTH,
            HEIGHT,
            &out_dir.join(format!("{stem}_preview.png")),
        );
        write_rgb(
            &buffers.map,
            MAP_WIDTH,
            MAP_HEIGHT,
            &out_dir.join(format!("{stem}_map.png")),
        );
        writeln!(
            html,
            "<figure><figcaption><b>{}</b><br><code>{}</code></figcaption>\
             <img src=\"{stem}_map.png\" width=\"300\"><img src=\"{stem}_preview.png\" width=\"318\"></figure>",
            shot.name,
            explorer.title()
        )
        .unwrap();
        println!("{} / {}: {}", A::NAME, shot.name, explorer.title());
    }
}

fn main() {
    let out_dir = std::env::args()
        .nth(1)
        .expect("usage: ui_screenshots <out dir>");
    let out_dir = Path::new(&out_dir);
    fs::create_dir_all(out_dir).unwrap();
    let mut html = String::from(
        "<!doctype html><meta charset=\"utf-8\"><title>Explorer screenshots</title>\
         <style>body{font-family:system-ui;background:#111;color:#ddd;margin:16px}\
         figure{display:inline-block;margin:8px;vertical-align:top}\
         figcaption{font-size:12px;max-width:620px;margin-bottom:4px}img{margin-right:4px}</style>\
         <h1>Explorer, rendered without windows</h1><p>Each shot: map window, then attractor window, \
         with the attractor window's title above.</p>\n",
    );
    macro_rules! shoot_all {
        ($($t:ty),*) => {
            $(screenshots::<$t>(out_dir, &mut html);)*
        };
    }
    attractors::all_attractors!(shoot_all);
    fs::write(out_dir.join("index.html"), html).unwrap();
}
