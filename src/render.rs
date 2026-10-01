use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::thread;

use crate::attractors::{format_params, Attractor};
use crate::sampler::{num_threads, Histogram, Sampler, View};

/// Steps taken per output pixel when saving, so bigger images get proportionally more points.
pub const SAVE_STEPS_PER_PIXEL: f64 = 8.0;

/// The preview has far fewer, bigger pixels than a print. Fine detail averages out differently
/// in each, so the preview's counts are scaled down by this to look like the print shrunk down
/// (found by eye with `examples/preview_vs_print.rs`).
const PREVIEW_MATCH: f64 = 0.5;

/// Fraction of lit pixels allowed to be fully saturated, so a few very hot pixels don't make
/// everything else dark.
const SATURATED_FRACTION: f64 = 0.005;

enum Gradient {
    Colorous(colorous::Gradient),
    /// `(position, rgb)` stops with positions increasing from 0.0 to 1.0
    Stops(&'static [(f64, [u8; 3])]),
}

pub struct Palette {
    pub name: &'static str,
    gradient: Gradient,
}

/// Every palette goes from the background colour (empty pixels) to the most visited pixels.
pub const PALETTES: &[Palette] = &[
    Palette {
        name: "Inferno",
        gradient: Gradient::Colorous(colorous::INFERNO),
    },
    Palette {
        name: "Aurora",
        gradient: Gradient::Stops(&[
            (0.0, [0, 0, 0]),
            (0.3, [0, 55, 60]),
            (0.6, [0, 165, 135]),
            (0.85, [110, 240, 190]),
            (1.0, [235, 255, 245]),
        ]),
    },
    Palette {
        name: "Ice",
        gradient: Gradient::Stops(&[
            (0.0, [2, 4, 12]),
            (0.3, [12, 36, 82]),
            (0.6, [44, 115, 180]),
            (0.85, [140, 210, 240]),
            (1.0, [250, 252, 255]),
        ]),
    },
    Palette {
        name: "Gold",
        gradient: Gradient::Stops(&[
            (0.0, [6, 4, 2]),
            (0.3, [70, 35, 8]),
            (0.6, [180, 110, 25]),
            (0.85, [245, 200, 95]),
            (1.0, [255, 250, 225]),
        ]),
    },
    Palette {
        name: "Neon",
        gradient: Gradient::Stops(&[
            (0.0, [4, 0, 10]),
            (0.3, [60, 0, 110]),
            (0.55, [210, 20, 180]),
            (0.8, [40, 210, 255]),
            (1.0, [240, 255, 255]),
        ]),
    },
    Palette {
        name: "Magma",
        gradient: Gradient::Colorous(colorous::MAGMA),
    },
    Palette {
        name: "Cubehelix",
        gradient: Gradient::Colorous(colorous::CUBEHELIX),
    },
    Palette {
        name: "Turbo",
        gradient: Gradient::Colorous(colorous::TURBO),
    },
    Palette {
        name: "Ink",
        gradient: Gradient::Stops(&[
            (0.0, [246, 242, 232]),
            (0.35, [170, 165, 160]),
            (0.7, [55, 60, 80]),
            (1.0, [8, 10, 20]),
        ]),
    },
    Palette {
        name: "Terracotta",
        gradient: Gradient::Stops(&[
            (0.0, [244, 236, 220]),
            (0.35, [222, 160, 115]),
            (0.7, [160, 70, 45]),
            (1.0, [50, 20, 15]),
        ]),
    },
];

impl Palette {
    fn rgb(&self, t: f64) -> [u8; 3] {
        let t = t.clamp(0.0, 1.0);
        match &self.gradient {
            Gradient::Colorous(g) => {
                let c = g.eval_continuous(t);
                [c.r, c.g, c.b]
            }
            Gradient::Stops(stops) => {
                let i = stops
                    .iter()
                    .position(|&(pos, _)| pos >= t)
                    .unwrap_or(0)
                    .max(1);
                let ((t0, c0), (t1, c1)) = (stops[i - 1], stops[i]);
                let f = (t - t0) / (t1 - t0);
                let (l0, l1) = (srgb_to_oklab(c0), srgb_to_oklab(c1));
                oklab_to_srgb([0, 1, 2].map(|j| l0[j] + f * (l1[j] - l0[j])))
            }
        }
    }
}

// sRGB <-> OKLab, from https://bottosson.github.io/posts/oklab/. Blending in OKLab keeps
// brightness and hue changing evenly along a gradient, where sRGB blends go muddy in the middle.

fn srgb_to_linear(c: u8) -> f64 {
    let c = c as f64 / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(c: f64) -> u8 {
    let c = c.clamp(0.0, 1.0);
    let c = if c <= 0.0031308 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (c * 255.0).round() as u8
}

fn srgb_to_oklab(rgb: [u8; 3]) -> [f64; 3] {
    let [r, g, b] = rgb.map(srgb_to_linear);
    let l = (0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b).cbrt();
    let m = (0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b).cbrt();
    let s = (0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b).cbrt();
    [
        0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
        1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
        0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s,
    ]
}

fn oklab_to_srgb([l, a, b]: [f64; 3]) -> [u8; 3] {
    let l_ = (l + 0.3963377774 * a + 0.2158037573 * b).powi(3);
    let m_ = (l - 0.1055613458 * a - 0.0638541728 * b).powi(3);
    let s_ = (l - 0.0894841775 * a - 1.2914855480 * b).powi(3);
    [
        4.0767416621 * l_ - 3.3077115913 * m_ + 0.2309699292 * s_,
        -1.2684380046 * l_ + 2.6097574011 * m_ - 0.3413193965 * s_,
        -0.0041960863 * l_ - 0.7034186147 * m_ + 1.7076147010 * s_,
    ]
    .map(linear_to_srgb)
}

/// How an attractor is coloured.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    /// Index into `PALETTES`
    pub palette: usize,
    /// Brightness is raised to this power, so above 1 darkens faint areas
    pub contrast: f64,
}

impl Default for Look {
    fn default() -> Self {
        Look {
            palette: 0,
            contrast: 1.0,
        }
    }
}

impl Look {
    pub fn palette(&self) -> &'static Palette {
        &PALETTES[self.palette % PALETTES.len()]
    }
}

const LUT_SIZE: usize = 1024;
static LUTS: LazyLock<Vec<[u32; LUT_SIZE]>> = LazyLock::new(|| {
    PALETTES
        .iter()
        .map(|p| {
            std::array::from_fn(|i| {
                let [r, g, b] = p.rgb(i as f64 / (LUT_SIZE - 1) as f64);
                ((r as u32) << 16) | ((g as u32) << 8) | b as u32
            })
        })
        .collect()
});

/// Colour at brightness `t` in [0, 1], packed as 0x00RRGGBB.
pub fn colour_u32(t: f64, look: &Look) -> u32 {
    let t = t.powf(look.contrast).clamp(0.0, 1.0);
    LUTS[look.palette % PALETTES.len()][(t * (LUT_SIZE - 1) as f64 + 0.5) as usize]
}

pub fn colour(t: f64, look: &Look) -> [u8; 3] {
    let [_, r, g, b] = colour_u32(t, look).to_be_bytes();
    [r, g, b]
}

/// Maps hit counts to brightnesses in [0, 1] on a log scale.
///
/// Counts are first converted to "hits per pixel of a saved print", so the result doesn't
/// depend on how many steps have been taken, and the preview looks like the print it'll save.
pub struct Tone {
    scale: f64,
    inv_norm: f64,
}

impl Tone {
    pub fn new(hist: &Histogram, extra_scale: f64) -> Self {
        let mut lit: Vec<u32> = hist.counts.iter().copied().filter(|&c| c > 0).collect();
        if lit.is_empty() {
            return Tone {
                scale: 0.0,
                inv_norm: 0.0,
            };
        }
        let idx = ((lit.len() - 1) as f64 * (1.0 - SATURATED_FRACTION)) as usize;
        let reference = *lit.select_nth_unstable(idx).1 as f64;
        let scale = extra_scale * SAVE_STEPS_PER_PIXEL * hist.counts.len() as f64
            / hist.steps.max(1) as f64;
        Tone {
            scale,
            inv_norm: 1.0 / (1.0 + reference * scale).ln(),
        }
    }

    pub fn brightness(&self, count: u32) -> f64 {
        ((1.0 + count as f64 * self.scale).ln() * self.inv_norm).min(1.0)
    }
}

/// Blend this frame's brightnesses into `prev` (see `noodle_factor`) and colour them into
/// `out`, split across all cores.
pub fn blend_and_colour(
    hist: &Histogram,
    prev: &mut [f64],
    noodle_factor: f64,
    look: &Look,
    out: &mut [u32],
) {
    assert!(prev.len() == hist.counts.len() && out.len() == hist.counts.len());
    let tone = Tone::new(hist, PREVIEW_MATCH);
    let tone = &tone;
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
                    *p = noodle_factor * tone.brightness(count) + (1.0 - noodle_factor) * *p;
                    *o = colour_u32(*p, look);
                }
            });
        }
    });
}

/// Render `attractor` at `size` into a PNG inside `directory`, returning the file path.
///
/// Continues from a copy of `sampler`, so the image is framed like the preview and the caller's
/// sampler is not advanced. Rows are streamed to the encoder, so memory is one `u32` per pixel.
pub fn save_png<A: Attractor>(
    attractor: &A,
    sampler: &Sampler,
    size: (usize, usize),
    view: &View,
    look: &Look,
    directory: &Path,
) -> io::Result<PathBuf> {
    let (width, height) = size;
    let total_steps = (width as f64 * height as f64 * SAVE_STEPS_PER_PIXEL) as usize;
    let mut hist = Histogram::new(width, height);
    let mut sampler = sampler.clone();

    let chunk = 50_000_000;
    while hist.steps < total_steps {
        sampler.step(
            attractor,
            chunk.min(total_steps - hist.steps),
            &mut hist,
            view,
        );
        print!(
            "\rSaving {width}x{height}: {:3.0}%",
            100.0 * hist.steps as f64 / total_steps as f64
        );
        io::stdout().flush()?;
    }
    println!();

    std::fs::create_dir_all(directory)?;
    let path = directory.join(format!(
        "{},size={}x{},iters={},palette={}.png",
        format_params(A::PARAM_NAMES, attractor.params()),
        width,
        height,
        hist.steps,
        look.palette().name.to_lowercase(),
    ));
    write_png(&hist, look, &path)?;
    Ok(path)
}

pub fn write_png(hist: &Histogram, look: &Look, path: &Path) -> io::Result<()> {
    let file = BufWriter::new(File::create(path)?);
    let mut encoder = png::Encoder::new(file, hist.width as u32, hist.height as u32);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(io::Error::other)?
        .into_stream_writer()
        .map_err(io::Error::other)?;

    let tone = Tone::new(hist, 1.0);
    let mut row = vec![0u8; hist.width * 3];
    for counts in hist.counts.chunks_exact(hist.width) {
        for (pixel, &count) in row.as_chunks_mut::<3>().0.iter_mut().zip(counts) {
            *pixel = colour(tone.brightness(count), look);
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
        let look = Look::default();
        blend_and_colour(&hist, &mut prev, 1.0, &look, &mut out);
        assert!(prev.iter().all(|&v| v == 0.0));
        assert!(out.iter().all(|&c| c == colour_u32(0.0, &look)));
    }

    #[test]
    fn lookup_table_matches_gradient() {
        for (i, palette) in PALETTES.iter().enumerate() {
            let look = Look {
                palette: i,
                contrast: 0.5,
            };
            for j in 0..=100 {
                let t = j as f64 / 100.0;
                let expected = palette.rgb(t.powf(0.5));
                let actual = colour(t, &look);
                for (a, e) in actual.iter().zip(expected) {
                    assert!(a.abs_diff(e) <= 2, "{} at {t}: {a} vs {e}", palette.name);
                }
            }
        }
    }

    #[test]
    fn stops_hit_their_endpoints() {
        for palette in PALETTES {
            if let Gradient::Stops(stops) = palette.gradient {
                assert_eq!(palette.rgb(0.0), stops[0].1, "{}", palette.name);
                assert_eq!(
                    palette.rgb(1.0),
                    stops[stops.len() - 1].1,
                    "{}",
                    palette.name
                );
            }
        }
    }

    #[test]
    fn oklab_round_trips() {
        for rgb in [[0, 0, 0], [255, 255, 255], [12, 200, 77], [250, 3, 140]] {
            assert_eq!(oklab_to_srgb(srgb_to_oklab(rgb)), rgb);
        }
    }

    #[test]
    fn brightness_ignores_number_of_steps() {
        let mut few = Histogram::new(2, 1);
        few.counts = vec![10, 20];
        few.steps = 30;
        let mut many = few.clone();
        many.counts = vec![100, 200];
        many.steps = 300;
        let (few_tone, many_tone) = (Tone::new(&few, 1.0), Tone::new(&many, 1.0));
        for (a, b) in few.counts.iter().zip(&many.counts) {
            let (a, b) = (few_tone.brightness(*a), many_tone.brightness(*b));
            assert!((a - b).abs() < 1e-9, "{a} vs {b}");
        }
    }
}
