//! Type-style token tests: size, line height, tracking, weight, family, tabular figures and the icon font.

use super::*;
use bevy::text::{FontFeatureTag, FontFeatures, LetterSpacing, LineHeight};

#[test]
fn a_type_token_applies_size_line_height_tracking_weight_and_family() {
    let mut app = app();
    let e = app
        .world_mut()
        .spawn((Text::new("Title"), TypeStyleToken(TypeScale::TitleLarge)))
        .id();
    app.update();
    let w = app.world();
    let font = w.get::<TextFont>(e).expect("font");
    assert_eq!(font.font_size, FontSize::Px(20.0));
    assert_eq!(font.weight, FontWeight(700));
    assert_eq!(font.font, crate::fonts::ui_font_source());
    assert_eq!(
        w.get::<LineHeight>(e),
        Some(&LineHeight::RelativeToFont(28.0 / 20.0))
    );
    assert_eq!(w.get::<LetterSpacing>(e), Some(&LetterSpacing::Px(0.0)));
}

#[test]
fn the_users_base_size_and_family_reach_the_text_in_the_next_frame_and_never_go_below_12_px() {
    let mut app = app();
    let title = app
        .world_mut()
        .spawn((Text::new("T"), TypeStyleToken(TypeScale::TitleLarge)))
        .id();
    let label = app
        .world_mut()
        .spawn((Text::new("l"), TypeStyleToken(TypeScale::Label)))
        .id();
    app.update();
    // 20 px base: every style grows by 20 / 14.
    app.insert_resource(crate::fonts::UiTypography {
        family: "OpenDyslexic".into(),
        base_px: 20.0,
    });
    app.update();
    let font = |app: &App, e| app.world().get::<TextFont>(e).expect("font").clone();
    let t = font(&app, title);
    assert_eq!(t.font_size, FontSize::Px(20.0 * 20.0 / 14.0));
    assert_eq!(
        t.font,
        FontSource::Families(format!("\"OpenDyslexic\", {}", crate::fonts::UI_FONT_STACK).into())
    );
    assert_eq!(
        font(&app, label).font_size,
        FontSize::Px(12.0 * 20.0 / 14.0)
    );
    // The smallest base size is 12 px: the label would shrink to 10.3 px and is held at 12.
    app.insert_resource(crate::fonts::UiTypography {
        family: String::new(),
        base_px: 12.0,
    });
    app.update();
    assert_eq!(font(&app, label).font_size, FontSize::Px(12.0));
    assert_eq!(font(&app, title).font, crate::fonts::ui_font_source());
}

#[test]
fn tabular_styles_switch_on_tabular_figures_and_other_styles_do_not() {
    let mut app = app();
    let speed = app
        .world_mut()
        .spawn((Text::new("48"), TypeStyleToken(TypeScale::NumericLarge)))
        .id();
    let body = app
        .world_mut()
        .spawn((Text::new("x"), TypeStyleToken(TypeScale::BodyMedium)))
        .id();
    app.update();
    let tabular = FontFeatures::builder()
        .enable(FontFeatureTag::TABULAR_FIGURES)
        .build();
    assert_eq!(
        app.world().get::<TextFont>(speed).expect("f").font_features,
        tabular
    );
    assert_eq!(
        app.world().get::<TextFont>(body).expect("f").font_features,
        FontFeatures::default()
    );
}

#[test]
fn the_icon_font_keeps_its_font_but_gets_the_size() {
    let mut app = app();
    let e = app
        .world_mut()
        .spawn((
            Text::new("x"),
            TextFont {
                font: crate::fonts::ICON_FONT.into(),
                ..default()
            },
            TypeStyleToken(TypeScale::BodyLarge),
        ))
        .id();
    app.update();
    let font = app.world().get::<TextFont>(e).expect("font");
    assert_eq!(font.font, FontSource::Handle(crate::fonts::ICON_FONT));
    assert_eq!(font.font_size, FontSize::Px(16.0));
}
