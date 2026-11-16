//! Image metrics for A/B comparisons of screenshots (`opts-tools image-metrics`).
//!
//! All values are computed on the sRGB-encoded pixel values scaled to `0..=1` (the numbers a
//! screenshot shows), so they are comparable with the measurements in
//! `docs/src/architecture/texture-quality.md`:
//!
//! - **saturation**: mean HSL saturation over all pixels;
//! - **luminance**: mean, standard deviation (the "contrast") and minimum of Rec. 709 luma;
//! - **laplacian**: standard deviation of the 4-neighbour Laplacian of the luma, a measure of
//!   high-frequency energy (detail *and* aliasing noise);
//! - **difference** of two images of equal size: mean absolute error per channel, RMSE, PSNR.

use image::RgbImage;

/// A rectangle of pixels (`x`, `y`, `w`, `h`) used to leave out HUD areas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Crop {
    /// Left edge.
    pub x: u32,
    /// Top edge.
    pub y: u32,
    /// Width.
    pub w: u32,
    /// Height.
    pub h: u32,
}

impl Crop {
    /// Parses `x,y,w,h`.
    ///
    /// # Errors
    /// A message when the text does not have four non-negative integers or has an empty area.
    pub fn parse(text: &str) -> Result<Self, String> {
        let parts: Vec<u32> = text
            .split(',')
            .map(|p| p.trim().parse::<u32>())
            .collect::<Result<_, _>>()
            .map_err(|e| format!("crop `{text}`: {e}"))?;
        match parts[..] {
            [x, y, w, h] if w > 0 && h > 0 => Ok(Self { x, y, w, h }),
            _ => Err(format!(
                "crop `{text}`: expected x,y,w,h with a non-empty area"
            )),
        }
    }

    /// Cuts the rectangle out of `img`.
    ///
    /// # Errors
    /// A message when the rectangle does not fit into the image.
    pub fn apply(self, img: &RgbImage) -> Result<RgbImage, String> {
        if self.x + self.w > img.width() || self.y + self.h > img.height() {
            return Err(format!(
                "crop {},{},{},{} exceeds the {}x{} image",
                self.x,
                self.y,
                self.w,
                self.h,
                img.width(),
                img.height()
            ));
        }
        Ok(image::imageops::crop_imm(img, self.x, self.y, self.w, self.h).to_image())
    }
}

/// The metrics of one image.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    /// Pixels measured.
    pub pixels: u64,
    /// Mean HSL saturation.
    pub saturation: f64,
    /// Mean luma.
    pub luma_mean: f64,
    /// Standard deviation of luma (global contrast).
    pub luma_std: f64,
    /// Darkest luma.
    pub luma_min: f64,
    /// Standard deviation of the luma Laplacian (high-frequency energy).
    pub laplacian: f64,
}

/// The difference of two images of equal size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Difference {
    /// Mean absolute difference per channel, `0..=1`.
    pub mean_abs: f64,
    /// Root mean square error per channel, `0..=1`.
    pub rmse: f64,
    /// Peak signal-to-noise ratio in dB (`inf` for identical images).
    pub psnr: f64,
}

fn luma(p: &image::Rgb<u8>) -> f64 {
    let [r, g, b] = p.0.map(|v| f64::from(v) / 255.0);
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

fn hsl_saturation(p: &image::Rgb<u8>) -> f64 {
    let [r, g, b] = p.0.map(|v| f64::from(v) / 255.0);
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let l = (max + min) / 2.0;
    let d = 1.0 - (2.0 * l - 1.0).abs();
    if d <= 1e-9 {
        0.0
    } else {
        (max - min) / d
    }
}

/// Measures `img`.
#[must_use]
pub fn measure(img: &RgbImage) -> Metrics {
    let (w, h) = (img.width() as usize, img.height() as usize);
    let lumas: Vec<f64> = img.pixels().map(luma).collect();
    let n = lumas.len().max(1) as f64;
    let luma_mean = lumas.iter().sum::<f64>() / n;
    let var = lumas.iter().map(|l| (l - luma_mean).powi(2)).sum::<f64>() / n;
    let luma_min = lumas.iter().copied().fold(1.0, f64::min);
    let saturation = img.pixels().map(hsl_saturation).sum::<f64>() / n;

    let mut lap = Vec::new();
    if w >= 3 && h >= 3 {
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let c = lumas[y * w + x];
                let v = lumas[(y - 1) * w + x]
                    + lumas[(y + 1) * w + x]
                    + lumas[y * w + x - 1]
                    + lumas[y * w + x + 1]
                    - 4.0 * c;
                lap.push(v);
            }
        }
    }
    let ln = lap.len().max(1) as f64;
    let lm = lap.iter().sum::<f64>() / ln;
    let laplacian = (lap.iter().map(|v| (v - lm).powi(2)).sum::<f64>() / ln).sqrt();

    Metrics {
        pixels: lumas.len() as u64,
        saturation,
        luma_mean,
        luma_std: var.sqrt(),
        luma_min,
        laplacian,
    }
}

/// Compares two images of the same size.
///
/// # Errors
/// A message when the sizes differ.
pub fn difference(a: &RgbImage, b: &RgbImage) -> Result<Difference, String> {
    if a.dimensions() != b.dimensions() {
        return Err(format!(
            "image sizes differ: {}x{} vs {}x{}",
            a.width(),
            a.height(),
            b.width(),
            b.height()
        ));
    }
    let (mut abs, mut sq, mut n) = (0.0, 0.0, 0u64);
    for (pa, pb) in a.pixels().zip(b.pixels()) {
        for c in 0..3 {
            let d = (f64::from(pa.0[c]) - f64::from(pb.0[c])) / 255.0;
            abs += d.abs();
            sq += d * d;
            n += 1;
        }
    }
    let n = n.max(1) as f64;
    let rmse = (sq / n).sqrt();
    let psnr = if rmse == 0.0 {
        f64::INFINITY
    } else {
        20.0 * (1.0 / rmse).log10()
    };
    Ok(Difference {
        mean_abs: abs / n,
        rmse,
        psnr,
    })
}

#[cfg(test)]
mod tests;
