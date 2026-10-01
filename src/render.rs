use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use colorous::Gradient;

use crate::attractors::{format_params, Attractor, Histogram, View};

const GRADIENT: Gradient = colorous::TURBO;

/// Steps taken per output pixel when saving, so bigger images get proportionally more points.
pub const SAVE_STEPS_PER_PIXEL: f64 = 1.0;

/// Colour of a pixel whose normalised density is `density`. The preview and saved images both
/// use this so they look the same.
pub fn colour(density: f64, power: f64) -> [u8; 3] {
    let c = GRADIENT.eval_continuous(density.powf(power));
    [c.r, c.g, c.b]
}

pub fn colour_u32(density: f64, power: f64) -> u32 {
    let [r, g, b] = colour(density, power);
    ((r as u32) << 16) | ((g as u32) << 8) | b as u32
}

/// Render `attractor` at `size` into a PNG inside `directory`, returning the file path.
///
/// Works on a copy of the attractor, so the caller's attractor is not advanced. Rows are streamed
/// to the encoder, so memory is one `u32` per pixel.
pub fn save_png<A: Attractor + Clone>(
    attractor: &A,
    size: (usize, usize),
    view: &View,
    power: f64,
    directory: &Path,
) -> io::Result<PathBuf> {
    let (width, height) = size;
    let total_steps = (width as f64 * height as f64 * SAVE_STEPS_PER_PIXEL) as usize;
    let mut hist = Histogram::new(width, height);
    let mut attractor = attractor.clone();

    let chunk = 10_000_000;
    print!("Stepping {total_steps} times ");
    while hist.steps < total_steps {
        attractor.step(chunk.min(total_steps - hist.steps), &mut hist, view);
        print!(".");
        io::stdout().flush()?;
    }
    println!();

    std::fs::create_dir_all(directory)?;
    let path = directory.join(format!(
        "{},size={}x{},iters={}.png",
        format_params(attractor.params()),
        width,
        height,
        hist.steps
    ));
    write_png(&hist, power, &path)?;
    Ok(path)
}

pub fn write_png(hist: &Histogram, power: f64, path: &Path) -> io::Result<()> {
    let file = BufWriter::new(File::create(path)?);
    let mut encoder = png::Encoder::new(file, hist.width as u32, hist.height as u32);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(io::Error::other)?
        .into_stream_writer()
        .map_err(io::Error::other)?;

    let inv_max = if hist.max == 0 {
        0.0
    } else {
        1.0 / hist.max as f64
    };
    let mut row = vec![0u8; hist.width * 3];
    for counts in hist.counts.chunks_exact(hist.width) {
        for (pixel, &count) in row.as_chunks_mut::<3>().0.iter_mut().zip(counts) {
            *pixel = colour(count as f64 * inv_max, power);
        }
        writer.write_all(&row)?;
    }
    writer.finish().map_err(io::Error::other)?;
    Ok(())
}
