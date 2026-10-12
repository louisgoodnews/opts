//! WCAG contrast arithmetic and the lightness search the accent generator uses to guarantee it.
//!
//! Everything here is pure (no ECS) so the contrast guarantees can be unit-tested over every
//! theme and preset.

use bevy::color::{Color, Oklcha};

/// Converts an sRGB colour component in `[0, 1]` to linear light.
fn srgb_to_linear(channel: f32) -> f32 {
    if channel <= 0.04045 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

/// The WCAG 2.1 relative luminance of a colour.
pub fn relative_luminance(color: Color) -> f32 {
    let srgba = color.to_srgba();
    0.2126 * srgb_to_linear(srgba.red)
        + 0.7152 * srgb_to_linear(srgba.green)
        + 0.0722 * srgb_to_linear(srgba.blue)
}

/// The WCAG 2.1 contrast ratio between two colours, `1.0` (none) to `21.0` (black on white).
///
/// Alpha is ignored: composite translucent colours over their backdrop first
/// ([`composite_over`]).
pub fn contrast_ratio(c1: Color, c2: Color) -> f32 {
    let l1 = relative_luminance(c1);
    let l2 = relative_luminance(c2);
    let (lighter, darker) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    (lighter + 0.05) / (darker + 0.05)
}

/// Composites `fg` (with its alpha) over the opaque `bg`; the result is opaque.
pub fn composite_over(fg: Color, bg: Color) -> Color {
    let f = fg.to_srgba();
    let b = bg.to_srgba();
    let a = f.alpha.clamp(0.0, 1.0);
    Color::srgb(
        f.red * a + b.red * (1.0 - a),
        f.green * a + b.green * (1.0 - a),
        f.blue * a + b.blue * (1.0 - a),
    )
}

/// The least contrast `fg` has against any of `backgrounds` (`f32::INFINITY` for an empty list).
pub fn min_contrast(fg: Color, backgrounds: &[Color]) -> f32 {
    backgrounds
        .iter()
        .map(|&bg| contrast_ratio(fg, bg))
        .fold(f32::INFINITY, f32::min)
}

/// Builds an opaque colour from OKLCH, reducing chroma until it fits the sRGB gamut (hue and
/// lightness are kept), so the result is always a valid display colour.
pub fn oklch(lightness: f32, chroma: f32, hue: f32) -> Color {
    let mut c = chroma.max(0.0);
    let lightness = lightness.clamp(0.0, 1.0);
    for _ in 0..40 {
        let srgb = Color::from(Oklcha::new(lightness, c, hue, 1.0)).to_srgba();
        let ok = |v: f32| (-0.0005..=1.0005).contains(&v);
        if ok(srgb.red) && ok(srgb.green) && ok(srgb.blue) {
            return Color::srgb(
                srgb.red.clamp(0.0, 1.0),
                srgb.green.clamp(0.0, 1.0),
                srgb.blue.clamp(0.0, 1.0),
            );
        }
        c *= 0.93;
    }
    Color::from(Oklcha::new(lightness, 0.0, hue, 1.0))
}

/// Returns `color` with its OKLCH lightness moved (in steps of `0.005`, keeping hue and chroma)
/// until it has at least `min` contrast against **every** background, searching towards white
/// when `lighter` is set and towards black otherwise.
///
/// If the search runs out of range the extreme (white or black) is returned, which is the best
/// contrast achievable in that direction. A colour that already passes is returned unchanged.
pub fn ensure_contrast(color: Color, backgrounds: &[Color], min: f32, lighter: bool) -> Color {
    if min_contrast(color, backgrounds) >= min {
        return color;
    }
    let lch: Oklcha = color.into();
    let step = if lighter { 0.005 } else { -0.005 };
    let mut l = lch.lightness;
    loop {
        l += step;
        if !(0.0..=1.0).contains(&l) {
            return if lighter { Color::WHITE } else { Color::BLACK };
        }
        let candidate = oklch(l, lch.chroma, lch.hue);
        if min_contrast(candidate, backgrounds) >= min {
            return candidate;
        }
    }
}

/// Picks the direction for [`ensure_contrast`]: towards white against dark backgrounds,
/// towards black against light ones (decided by the mean luminance of the backgrounds).
pub fn prefers_lighter(backgrounds: &[Color]) -> bool {
    if backgrounds.is_empty() {
        return true;
    }
    let mean = backgrounds
        .iter()
        .map(|&c| relative_luminance(c))
        .sum::<f32>()
        / backgrounds.len() as f32;
    mean < 0.18
}

/// Parses `#RRGGBB` (the `#` is optional) into an opaque colour.
pub fn parse_hex(hex: &str) -> Option<Color> {
    let hex = hex.trim().trim_start_matches('#');
    if hex.len() != 6 || !hex.is_ascii() {
        return None;
    }
    let channel = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    Some(Color::srgb_u8(channel(0)?, channel(2)?, channel(4)?))
}

/// Formats an opaque colour as `#RRGGBB`.
pub fn to_hex(color: Color) -> String {
    let c = color.to_srgba();
    let b = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02X}{:02X}{:02X}", b(c.red), b(c.green), b(c.blue))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn black_on_white_is_21() {
        assert!((contrast_ratio(Color::BLACK, Color::WHITE) - 21.0).abs() < 0.01);
        assert!((contrast_ratio(Color::WHITE, Color::WHITE) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn ensure_contrast_reaches_the_minimum_in_both_directions() {
        let dark = Color::srgb(0.07, 0.07, 0.09);
        let light = Color::srgb(0.99, 0.97, 0.99);
        let mid = Color::srgb(0.5, 0.5, 0.5);
        let lifted = ensure_contrast(mid, &[dark], 4.5, true);
        assert!(contrast_ratio(lifted, dark) >= 4.5);
        let sunk = ensure_contrast(mid, &[light], 4.5, false);
        assert!(contrast_ratio(sunk, light) >= 4.5);
    }

    #[test]
    fn ensure_contrast_leaves_passing_colours_alone() {
        assert_eq!(
            ensure_contrast(Color::WHITE, &[Color::BLACK], 7.0, true),
            Color::WHITE
        );
    }

    #[test]
    fn composite_blends_by_alpha() {
        let c = composite_over(Color::srgba(1.0, 1.0, 1.0, 0.5), Color::BLACK).to_srgba();
        assert!((c.red - 0.5).abs() < 1e-4);
    }

    #[test]
    fn hex_roundtrip() {
        let c = parse_hex("#0061A4").expect("valid");
        assert_eq!(to_hex(c), "#0061A4");
        assert!(parse_hex("#12345").is_none());
        assert!(parse_hex("zzzzzz").is_none());
    }
}
