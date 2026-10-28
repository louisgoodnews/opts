//! Golden screenshots without a GPU: a software painter for the laid-out UI tree and a
//! tolerance-based image comparison.
//!
//! **What this is.** [`paint`] draws what Bevy's UI layout computed for a headless app: node
//! backgrounds with rounded corners, borders and the real glyphs of every text (the font atlas
//! Bevy rasterises on the CPU), in paint order. It is deterministic and needs no window, GPU or
//! display, so it runs in CI. **What it is not.** It is not a pixel-exact copy of the GPU
//! renderer: images, shadows, outlines (the focus ring), gradients and clipping details are not
//! painted. It guards layout, colour and text regressions; the GPU screenshots of the control
//! protocol (`output.screenshot`) remain the proof of the real frame, which is an open item for a
//! human run.
//!
//! **Goldens contain fixture content only** (self-authored catalogue and texts; ADR-0002 forbids
//! OMSI content in the repository). [`check`] compares with a [`Tolerance`] and, with
//! `UPDATE_GOLDEN=1`, (re)writes the golden file.

use std::path::{Path, PathBuf};

use image::{Rgba, RgbaImage};

mod paint;
pub use paint::*;

/// How far an image may differ from its golden and still pass.
#[derive(Debug, Clone, Copy)]
pub struct Tolerance {
    /// Largest allowed difference per colour channel (0 to 255) before a pixel counts as different.
    pub channel: u8,
    /// Largest allowed share of different pixels (0.0 to 1.0).
    pub fraction: f32,
}

impl Default for Tolerance {
    fn default() -> Self {
        Self {
            channel: 6,
            fraction: 0.002,
        }
    }
}

/// Result of [`compare`].
#[derive(Debug, Clone, PartialEq)]
pub struct Difference {
    /// Pixels beyond the channel tolerance.
    pub different: u64,
    /// Total pixels compared.
    pub total: u64,
    /// Largest channel difference seen.
    pub max_channel: u8,
    /// The two images have different sizes (then nothing else is meaningful).
    pub size_mismatch: bool,
}

impl Difference {
    /// Share of different pixels.
    pub fn fraction(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            self.different as f32 / self.total as f32
        }
    }
}

/// Compares two images.
pub fn compare(actual: &RgbaImage, golden: &RgbaImage, tol: Tolerance) -> Difference {
    if actual.dimensions() != golden.dimensions() {
        return Difference {
            different: u64::MAX,
            total: 0,
            max_channel: 255,
            size_mismatch: true,
        };
    }
    let mut different = 0;
    let mut max_channel = 0u8;
    for (a, g) in actual.pixels().zip(golden.pixels()) {
        let d =
            a.0.iter()
                .zip(g.0.iter())
                .map(|(x, y)| x.abs_diff(*y))
                .max()
                .unwrap_or(0);
        max_channel = max_channel.max(d);
        if d > tol.channel {
            different += 1;
        }
    }
    Difference {
        different,
        total: u64::from(actual.width()) * u64::from(actual.height()),
        max_channel,
        size_mismatch: false,
    }
}

/// Does `d` pass `tol`?
pub fn passes(d: &Difference, tol: Tolerance) -> bool {
    !d.size_mismatch && d.fraction() <= tol.fraction
}

/// Compares `actual` with `dir/name.png`. With `UPDATE_GOLDEN=1` the golden is written instead.
/// On a mismatch the actual image and a difference image are written to
/// `$OPTS_GOLDEN_OUT` (default: `dir/_failed`) and an error describing the numbers is returned.
pub fn check(dir: &Path, name: &str, actual: &RgbaImage, tol: Tolerance) -> Result<(), String> {
    let path = dir.join(format!("{name}.png"));
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        write_png(actual, &path)?;
        return Ok(());
    }
    let golden = image::open(&path)
        .map_err(|e| {
            format!(
                "golden {} missing or unreadable ({e}); run with UPDATE_GOLDEN=1 to create it",
                path.display()
            )
        })?
        .to_rgba8();
    let d = compare(actual, &golden, tol);
    if passes(&d, tol) {
        return Ok(());
    }
    let out: PathBuf = std::env::var_os("OPTS_GOLDEN_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| dir.join("_failed"));
    let _ = std::fs::create_dir_all(&out);
    let _ = actual.save(out.join(format!("{name}.actual.png")));
    if !d.size_mismatch {
        let mut diff = RgbaImage::new(actual.width(), actual.height());
        for (x, y, p) in diff.enumerate_pixels_mut() {
            let a = actual.get_pixel(x, y);
            let g = golden.get_pixel(x, y);
            let delta =
                a.0.iter()
                    .zip(g.0.iter())
                    .map(|(p, q)| p.abs_diff(*q))
                    .max()
                    .unwrap_or(0);
            *p = if delta > tol.channel {
                Rgba([255, 0, 0, 255])
            } else {
                Rgba([g.0[0] / 2 + 64, g.0[1] / 2 + 64, g.0[2] / 2 + 64, 255])
            };
        }
        let _ = diff.save(out.join(format!("{name}.diff.png")));
    }
    Err(format!(
        "{name}: {} of {} pixels differ (max channel delta {}, allowed {:.3}%), size mismatch {}; see {}",
        d.different,
        d.total,
        d.max_channel,
        tol.fraction * 100.0,
        d.size_mismatch,
        out.display()
    ))
}

/// Writes `img` as a PNG with the best compression (goldens are checked in; flat colours shrink a lot).
fn write_png(img: &RgbaImage, path: &Path) -> Result<(), String> {
    use image::codecs::png::{CompressionType, FilterType, PngEncoder};
    use image::ImageEncoder;
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    PngEncoder::new_with_quality(
        std::io::BufWriter::new(file),
        CompressionType::Best,
        FilterType::Adaptive,
    )
    .write_image(
        img.as_raw(),
        img.width(),
        img.height(),
        image::ExtendedColorType::Rgba8,
    )
    .map_err(|e| e.to_string())
}
