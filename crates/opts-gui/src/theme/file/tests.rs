//! Unit tests for `super` (`file.rs`).

use super::*;

use super::*;
use crate::theme::contrast::contrast_ratio;

#[test]
fn a_minimal_file_loads_and_is_sound() {
    let t = parse_theme_file(r#"(version: 1, name: "Plain")"#).expect("loads");
    assert_eq!(t.name, "Plain");
    for kind in [ThemeKind::Dark, ThemeKind::Light, ThemeKind::HighContrast] {
        assert!(audit(t.scheme(kind)).is_empty());
    }
    assert!(t.corrected.is_empty());
}

#[test]
fn an_accent_and_overrides_apply() {
    let t = parse_theme_file(
        r##"(version: 1, name: "H", accent: Some("#2A7F62"), dark: {"Surface": "#0E1A22"})"##,
    )
    .expect("loads");
    assert_eq!(
        to_hex(t.scheme(ThemeKind::Dark).get(ColorRole::Surface)),
        "#0E1A22"
    );
    assert!(t.accent.is_some());
}

#[test]
fn a_bad_pair_is_corrected_and_reported() {
    // yellow text on the light-blue dark-theme primary: about 1.3:1
    let t = parse_theme_file(r##"(version: 1, name: "Loud", dark: {"OnPrimary": "#FFFF00"})"##)
        .expect("auto-corrected");
    let s = t.scheme(ThemeKind::Dark);
    assert!(contrast_ratio(s.get(ColorRole::OnPrimary), s.get(ColorRole::Primary)) >= 4.5);
    assert!(
        t.corrected
            .contains(&(ThemeKind::Dark, ColorRole::OnPrimary))
    );
    assert!(audit(s).is_empty());
    // only the dark theme was touched
    assert!(t.corrected.iter().all(|(k, _)| *k == ThemeKind::Dark));
}

#[test]
fn colours_that_cannot_work_are_rejected() {
    // a near-white surface in the dark theme: no single text colour reads on it and on the
    // dark containers at the same time
    let e = parse_theme_file(r##"(version: 1, name: "Bright", dark: {"Surface": "#F0F0F0"})"##)
        .expect_err("unfixable");
    assert!(
        matches!(
            e,
            ThemeFileError::Unfixable {
                kind: ThemeKind::Dark,
                ..
            }
        ),
        "{e}"
    );
}

#[test]
fn broken_files_are_rejected_with_a_reason() {
    for (text, check) in [
        ("(version: 2, name: \"x\")", "version"),
        ("garbage", "not a valid theme file"),
        (
            "(version: 1, name: \"x\", dark: {\"Nope\": \"#000000\"})",
            "unknown colour role",
        ),
        (
            "(version: 1, name: \"x\", dark: {\"Surface\": \"red\"})",
            "#RRGGBB",
        ),
        ("(version: 1, name: \"x\", accent: Some(\"12\"))", "#RRGGBB"),
        (
            "(version: 1, name: \"x\", colour: 3)",
            "not a valid theme file",
        ),
    ] {
        let e = parse_theme_file(text).expect_err(text).to_string();
        assert!(e.contains(check), "{text}: {e}");
    }
}

#[test]
fn role_names_are_the_variant_names() {
    assert_eq!(role_from_name("OnPrimary"), Some(ColorRole::OnPrimary));
    assert_eq!(role_from_name("onprimary"), None);
    for r in ColorRole::all() {
        assert_eq!(role_from_name(&format!("{r:?}")), Some(*r));
    }
}
