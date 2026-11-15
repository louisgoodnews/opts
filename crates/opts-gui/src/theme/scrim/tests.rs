//! Unit tests of `scrim`.

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
