//! The scrim over a photograph: how dark (or light) it must be for text on top of it to stay
//! readable, computed from the theme and from the image itself instead of a fixed alpha.
//!
//! Text colours come from the theme (light on dark themes, dark on light ones). The scrim colour is
//! the theme's `Scrim` (black) on dark themes and its `Surface` on light ones; its alpha is the
//! smallest one for which the text still reaches the required contrast over the **worst-case**
//! pixel of the image (the 95th percentile of luminance for light text, the 5th for dark text).

use bevy::color::{Alpha, Color};

use super::color::{ColorRole, ColorScheme};
use super::contrast::{composite_over, contrast_ratio, relative_luminance};

/// Lightness statistics of an image (relative luminance, `0.0` black to `1.0` white).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImageLuminance {
    /// The 5th percentile: the dark end of the image.
    pub p05: f32,
    /// The 95th percentile: the bright end of the image.
    pub p95: f32,
}

impl Default for ImageLuminance {
    /// Unknown image: assume the worst, from black to white.
    fn default() -> Self {
        Self { p05: 0.0, p95: 1.0 }
    }
}

/// Measures an 8-bit RGBA (or RGB with `channels == 3`) image, sampling at most ~65 000 pixels.
///
/// An empty or malformed buffer gives the worst-case default.
pub fn luminance_stats(pixels: &[u8], channels: usize) -> ImageLuminance {
    if channels < 3 || pixels.len() < channels {
        return ImageLuminance::default();
    }
    let count = pixels.len() / channels;
    let step = (count / 65_000).max(1);
    let mut lums: Vec<f32> = pixels
        .chunks_exact(channels)
        .step_by(step)
        .map(|p| relative_luminance(Color::srgb_u8(p[0], p[1], p[2])))
        .collect();
    if lums.is_empty() {
        return ImageLuminance::default();
    }
    lums.sort_by(|a, b| a.total_cmp(b));
    let at = |q: f32| lums[((lums.len() - 1) as f32 * q).round() as usize];
    ImageLuminance {
        p05: at(0.05),
        p95: at(0.95),
    }
}

/// A grey with the given relative luminance.
fn grey(luminance: f32) -> Color {
    let l = luminance.clamp(0.0, 1.0);
    let s = if l <= 0.003_130_8 {
        l * 12.92
    } else {
        1.055 * l.powf(1.0 / 2.4) - 0.055
    };
    Color::srgb(s, s, s)
}

/// The scrim colour (with alpha) that keeps `text` readable at `min_contrast` over an image with
/// the given statistics.
///
/// The alpha is at least `floor` (a calm veil even over dark images) and at most `0.95`.
pub fn scrim_for_text(
    scheme: &ColorScheme,
    text: ColorRole,
    stats: ImageLuminance,
    min_contrast: f32,
    floor: f32,
) -> Color {
    let text_color = scheme.get(text);
    let light_text = relative_luminance(text_color) > 0.18;
    let base = if scheme.is_dark() {
        scheme.get(ColorRole::Scrim)
    } else {
        scheme.get(ColorRole::Surface)
    };
    let worst = grey(if light_text { stats.p95 } else { stats.p05 });
    let mut alpha = floor.clamp(0.0, 0.95);
    while alpha < 0.95 {
        let over = composite_over(base.with_alpha(alpha), worst);
        if contrast_ratio(text_color, over) >= min_contrast {
            break;
        }
        alpha += 0.01;
    }
    base.with_alpha(alpha.min(0.95))
}

#[cfg(test)]
mod tests;
