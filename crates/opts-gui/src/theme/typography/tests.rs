//! Unit tests of `typography`.

use super::*;

#[test]
fn the_scale_has_nine_styles_none_below_the_floor() {
    let typography = TypographyTokens::default();
    assert_eq!(TypeScale::all().len(), 9);
    for &scale in TypeScale::all() {
        let style = typography.get(scale);
        assert!(style.font_size >= MIN_TEXT_PX, "{scale:?}");
        assert!(style.line_height >= style.font_size, "{scale:?}");
    }
}

#[test]
fn the_scale_matches_the_design_language() {
    let t = TypographyTokens::default();
    let sizes: Vec<f32> = TypeScale::all()
        .iter()
        .map(|&s| t.get(s).font_size)
        .collect();
    assert_eq!(
        sizes,
        [40.0, 28.0, 20.0, 16.0, 16.0, 14.0, 12.0, 48.0, 32.0]
    );
    assert!(t.get(TypeScale::NumericLarge).tabular);
    assert!(!t.get(TypeScale::BodyMedium).tabular);
}

#[test]
fn deprecated_aliases_land_on_a_scale_style_and_never_below_the_floor() {
    let t = TypographyTokens::default();
    for old in [
        TypeScale::DisplayLarge,
        TypeScale::DisplayMedium,
        TypeScale::DisplaySmall,
        TypeScale::HeadlineLarge,
        TypeScale::HeadlineMedium,
        TypeScale::HeadlineSmall,
        TypeScale::TitleSmall,
        TypeScale::LabelLarge,
        TypeScale::LabelMedium,
        TypeScale::LabelSmall,
        TypeScale::BodySmall,
    ] {
        assert!(TypeScale::all().contains(&old.canonical()), "{old:?}");
        assert!(t.resolve(old).font_size >= MIN_TEXT_PX, "{old:?}");
    }
    assert_eq!(t.resolve(TypeScale::LabelLarge).weight, 500);
    assert_eq!(t.resolve(TypeScale::BodyMedium).weight, 400);
}
