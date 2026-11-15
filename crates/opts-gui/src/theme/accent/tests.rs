//! Unit tests of `accent`.

use crate::theme::contrast::parse_hex;
use super::*;
use crate::theme::contrast::contrast_ratio;

#[test]
fn presets_parse_by_key() {
    for p in AccentPreset::ALL {
        assert_eq!(AccentPreset::parse(p.key()), Some(p));
    }
    assert_eq!(AccentPreset::parse("Pink"), None);
    assert_eq!(AccentPreset::default(), AccentPreset::TransitBlue);
}

#[test]
fn a_very_bright_and_a_very_dark_seed_both_read() {
    for kind in [ThemeKind::Dark, ThemeKind::Light, ThemeKind::HighContrast] {
        for seed in ["#FFFF00", "#000000", "#FFFFFF", "#808080", "#00FF00"] {
            let mut scheme = ColorScheme::for_kind(kind, AccentPreset::default().color());
            apply_accent(&mut scheme, parse_hex(seed).expect("hex"));
            let r = contrast_ratio(
                scheme.get(ColorRole::Primary),
                scheme.get(ColorRole::OnPrimary),
            );
            assert!(r >= text_min(kind), "{kind:?} {seed}: {r:.2}");
        }
    }
}
