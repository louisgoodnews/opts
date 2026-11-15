//! Unit tests of `fonts`.

use super::*;

fn covers(font: &[u8], ch: char) -> bool {
    use ab_glyph::Font as _;
    ab_glyph::FontRef::try_from_slice(font).is_ok_and(|f| f.glyph_id(ch).0 != 0)
}

#[test]
fn a_chosen_family_goes_in_front_of_the_stack_and_odd_names_are_ignored() {
    assert_eq!(ui_font_source_for(""), ui_font_source());
    assert_eq!(ui_font_source_for("Noto Sans"), ui_font_source());
    assert_eq!(ui_font_source_for("a, b"), ui_font_source());
    let f = ui_font_source_for("Atkinson Hyperlegible");
    assert_eq!(
        f,
        FontSource::Families("\"Atkinson Hyperlegible\", Noto Sans, DejaVu Sans".into())
    );
    assert!(is_ui_font(&f) && is_ui_font(&ui_font_source()));
    assert!(!is_ui_font(&FontSource::Family(
        "Atkinson Hyperlegible".into()
    )));
}

#[test]
fn the_base_size_scales_the_scale_and_keeps_the_floor() {
    let t = UiTypography {
        family: String::new(),
        base_px: 20.0,
    };
    assert!((t.size(14.0) - 20.0).abs() < 1e-4);
    assert!((t.size(40.0) - 40.0 * 20.0 / 14.0).abs() < 1e-3);
    let small = UiTypography {
        family: String::new(),
        base_px: 12.0,
    };
    assert_eq!(small.size(12.0), 12.0, "the floor holds");
    assert_eq!(
        UiTypography::default().size(16.0),
        16.0,
        "14 px is the identity"
    );
}

#[test]
fn the_stack_covers_german_text_and_ui_symbols() {
    // Each character must come from the UI family or, failing that, the fallback.
    for ch in "äöüÄÖÜß€°·•–—−μ✓✕✖ℹ↺▶▼●←↑→↓↖↗↘↙„“".chars()
    {
        assert!(
            covers(UI_REGULAR, ch) || covers(DEJAVU_SANS, ch),
            "neither Noto Sans nor DejaVu Sans has U+{:04X} {ch}",
            ch as u32
        );
    }
    // The text of the product itself needs no fallback.
    for ch in "äöüÄÖÜß€°·–—„“".chars() {
        assert!(covers(UI_REGULAR, ch), "Noto Sans lacks {ch}");
    }
}

#[test]
fn all_weights_are_distinct_faces_of_one_family() {
    assert_ne!(UI_REGULAR.len(), UI_MEDIUM.len());
    assert_ne!(UI_MEDIUM.len(), UI_BOLD.len());
    for bytes in [UI_REGULAR, UI_MEDIUM, UI_BOLD, MONO_REGULAR, DEJAVU_SANS] {
        assert!(ab_glyph::FontRef::try_from_slice(bytes).is_ok());
    }
}

#[test]
fn material_icons_cover_every_named_icon() {
    for (name, icon) in crate::icons::ALL {
        let ch = icon.chars().next().expect("icon is one char");
        assert!(covers(MATERIAL_ICONS, ch), "Material Icons lacks {name}");
    }
}

#[test]
fn the_default_and_the_stack_are_recognised_but_the_icon_font_is_not() {
    assert!(is_ui_font(&FontSource::default()));
    assert!(is_ui_font(&ui_font_source()));
    assert!(!is_ui_font(&FontSource::Handle(ICON_FONT)));
    assert!(!is_ui_font(&FontSource::Family("Anything".into())));
}
