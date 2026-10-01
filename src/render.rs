use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::thread;

use colorous::Gradient;

use crate::attractors::{format_params, Attractor};
use crate::sampler::{num_threads, Histogram, Sampler, View};

const GRADIENT: Gradient = colorous::TURBO;

/// Steps taken per output pixel when saving, so bigger images get proportionally more points.
pub const SAVE_STEPS_PER_PIXEL: f64 = 1.0;

const LUT_SIZE: usize = 1024;
static GRADIENT_LUT: LazyLock<[u32; LUT_SIZE]> = LazyLock::new(|| {
    std::array::from_fn(|i| {
        let c = GRADIENT.eval_continuous(i as f64 / (LUT_SIZE - 1) as f64);
        ((c.r as u32) << 16) | ((c.g as u32) << 8) | c.b as u32
    })
});

/// Colour of a pixel whose normalised density is `density`, packed as 0x00RRGGBB. The preview
/// and saved images both use this so they look the same.
pub fn colour_u32(density: f64, power: f64) -> u32 {
    let t = density.powf(power).clamp(0.0, 1.0);
    GRADIENT_LUT[(t * (LUT_SIZE - 1) as f64 + 0.5) as usize]
}

pub fn colour(density: f64, power: f64) -> [u8; 3] {
    let [_, r, g, b] = colour_u32(density, power).to_be_bytes();
    [r, g, b]
}

/// Blend this frame's normalised densities into `prev` (see `noodle_factor`) and colour them into
/// `out`, split across all cores.
pub fn blend_and_colour(
    hist: &Histogram,
    prev: &mut [f64],
    noodle_factor: f64,
    power: f64,
    out: &mut [u32],
) {
    assert!(prev.len() == hist.counts.len() && out.len() == hist.counts.len());
    let inv_max = if hist.max == 0 {
        0.0
    } else {
        1.0 / hist.max as f64
    };
    let chunk = hist.counts.len().div_ceil(num_threads());
    thread::scope(|s| {
        for ((counts, prev), out) in hist
            .counts
            .chunks(chunk)
            .zip(prev.chunks_mut(chunk))
            .zip(out.chunks_mut(chunk))
        {
            s.spawn(move || {
                for ((&count, p), o) in counts.iter().zip(prev).zip(out) {
                    *p = noodle_factor * (count as f64 * inv_max) + (1.0 - noodle_factor) * *p;
                    *o = colour_u32(*p, power);
                }
            });
        }
    });
}

/// Render `attractor` at `size` into a PNG inside `directory`, returning the file path.
///
/// Uses its own walkers, so the caller's sampler is not advanced. Rows are streamed
/// to the encoder, so memory is one `u32` per pixel.
pub fn save_png<A: Attractor>(
    attractor: &A,
    size: (usize, usize),
    view: &View,
    power: f64,
    directory: &Path,
) -> io::Result<PathBuf> {
    let (width, height) = size;
    let total_steps = (width as f64 * height as f64 * SAVE_STEPS_PER_PIXEL) as usize;
    let mut hist = Histogram::new(width, height);
    let mut sampler = Sampler::default();

    let chunk = 10_000_000;
    print!("Stepping {total_steps} times ");
    while hist.steps < total_steps {
        sampler.step(
            attractor,
            chunk.min(total_steps - hist.steps),
            &mut hist,
            view,
        );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_histogram_colours_to_background() {
        let hist = Histogram::new(4, 4);
        let mut prev = vec![0.5; 16];
        let mut out = vec![1u32; 16];
        blend_and_colour(&hist, &mut prev, 1.0, 0.15, &mut out);
        assert!(prev.iter().all(|&v| v == 0.0));
        assert!(out.iter().all(|&c| c == colour_u32(0.0, 0.15)));
    }

    #[test]
    fn lookup_table_matches_gradient() {
        for i in 0..=100 {
            let d = i as f64 / 100.0;
            let c = GRADIENT.eval_continuous(d.powf(0.5));
            let [r, g, b] = colour(d, 0.5);
            for (a, b) in [(r, c.r), (g, c.g), (b, c.b)] {
                assert!(a.abs_diff(b) <= 2, "density {d}: {a} vs {b}");
            }
        }
    }
}
