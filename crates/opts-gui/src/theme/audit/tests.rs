//! Unit tests of `audit`.

use super::*;
use crate::theme::accent::AccentPreset;
use crate::theme::mode::ThemeKind;

const KINDS: [ThemeKind; 3] = [ThemeKind::Dark, ThemeKind::Light, ThemeKind::HighContrast];

#[test]
fn every_theme_and_preset_passes_every_pair() {
    let mut checked = 0;
    for kind in KINDS {
        for preset in AccentPreset::ALL {
            let scheme = ColorScheme::for_kind(kind, preset.color());
            let violations = audit(&scheme);
            assert!(
                violations.is_empty(),
                "{kind:?} / {preset:?}: {} violations, first {:?}",
                violations.len(),
                violations.first()
            );
            checked += rules().len();
        }
    }
    assert!(checked > 1000, "{checked} pairs checked");
}

#[test]
fn the_default_constructors_are_sound() {
    for s in [
        ColorScheme::dark(),
        ColorScheme::light(),
        ColorScheme::high_contrast(),
    ] {
        assert!(audit(&s).is_empty(), "{:?}", audit(&s).first());
    }
}

#[test]
fn the_old_amber_pair_fails_and_is_corrected() {
    // light theme, raw amber primary with white text: 2.15:1 (audit finding UI-F09)
    let mut s = ColorScheme::light();
    s.set(ColorRole::Primary, Color::srgb(0.96, 0.62, 0.04));
    s.set(ColorRole::OnPrimary, Color::WHITE);
    assert!(audit(&s)
        .iter()
        .any(|v| v.rule.bg == ColorRole::Primary && v.ratio < 3.0));
    let changed = auto_correct(&mut s);
    assert!(changed.contains(&ColorRole::OnPrimary) || changed.contains(&ColorRole::Primary));
    assert!(audit(&s).is_empty(), "{:?}", audit(&s).first());
}

#[test]
fn high_contrast_reaches_seven_to_one_for_body_text() {
    let s = ColorScheme::high_contrast();
    for bg in SURFACES {
        assert!(contrast_ratio(s.get(ColorRole::OnSurface), s.get(bg)) >= 7.0);
        assert!(contrast_ratio(s.get(ColorRole::Outline), s.get(bg)) >= 4.5);
    }
}
