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
mod tests {
    use super::*;

    fn flat(rgb: u8, pixels: usize) -> Vec<u8> {
        (0..pixels).flat_map(|_| [rgb, rgb, rgb, 255]).collect()
    }

    #[test]
    fn stats_of_flat_images() {
        let white = luminance_stats(&flat(255, 100), 4);
        let black = luminance_stats(&flat(0, 100), 4);
        assert!(white.p05 > 0.99 && white.p95 > 0.99);
        assert!(black.p05 < 0.01 && black.p95 < 0.01);
        assert_eq!(luminance_stats(&[], 4), ImageLuminance::default());
    }

    #[test]
    fn text_stays_readable_over_a_bright_and_a_dark_image_in_every_theme() {
        let bright = luminance_stats(&flat(255, 400), 4);
        let dark = luminance_stats(&flat(0, 400), 4);
        // a busy photograph: half white, half black
        let mixed: Vec<u8> = flat(255, 200).into_iter().chain(flat(0, 200)).collect();
        let mixed = luminance_stats(&mixed, 4);
        for scheme in [
            ColorScheme::dark(),
            ColorScheme::light(),
            ColorScheme::high_contrast(),
        ] {
            for (name, stats) in [("bright", bright), ("dark", dark), ("mixed", mixed)] {
                for text in [ColorRole::OnSurface, ColorRole::OnSurfaceVariant] {
                    let scrim = scrim_for_text(&scheme, text, stats, 4.5, 0.2);
                    // check against both extremes of the image, not only the one it was made for
                    for backdrop in [stats.p05, stats.p95] {
                        let over = composite_over(scrim, grey(backdrop));
                        let ratio = contrast_ratio(scheme.get(text), over);
                        // the worst-case end is guaranteed; the other end only has to be sane
                        let worst_end = if relative_luminance(scheme.get(text)) > 0.18 {
                            stats.p95
                        } else {
                            stats.p05
                        };
                        if (backdrop - worst_end).abs() < 1e-6 {
                            assert!(
                                ratio >= 4.5,
                                "{:?} {name} {text:?}: {ratio:.2}",
                                scheme.kind
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn a_darker_image_needs_a_lighter_scrim() {
        let s = ColorScheme::dark();
        let a = |lum| {
            scrim_for_text(
                &s,
                ColorRole::OnSurface,
                ImageLuminance { p05: 0.0, p95: lum },
                4.5,
                0.1,
            )
            .to_srgba()
            .alpha
        };
        assert!(a(1.0) > a(0.2));
        assert!(a(0.0) <= 0.11, "a black image needs only the floor");
    }

    #[test]
    fn the_scrim_has_the_themes_colour() {
        let dark = scrim_for_text(
            &ColorScheme::dark(),
            ColorRole::OnSurface,
            ImageLuminance::default(),
            4.5,
            0.2,
        );
        let light = scrim_for_text(
            &ColorScheme::light(),
            ColorRole::OnSurface,
            ImageLuminance::default(),
            4.5,
            0.2,
        );
        assert!(relative_luminance(dark) < 0.01, "dark themes veil in black");
        assert!(
            relative_luminance(light) > 0.9,
            "light themes veil in white"
        );
    }
}
