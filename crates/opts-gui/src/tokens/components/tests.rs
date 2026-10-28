//! Unit tests for `super` (`components.rs`).

use super::*;
use crate::theme::ColorScheme;

#[test]
fn test_theme_change_updates_entities() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<MaterialTheme>()
        .add_systems(
            Update,
            (
                sync_background_roles,
                sync_border_roles,
                sync_foreground_roles,
                sync_radius_tokens,
                sync_typestyle_tokens,
            ),
        );

    // Spawn test entity container with token components and text child
    let entity = app
        .world_mut()
        .spawn((
            Node::default(),
            BackgroundRole(ColorRole::Primary),
            BorderRole(ColorRole::Outline),
            RadiusToken(RadiusLevel::Medium),
        ))
        .id();

    let text_child = app
        .world_mut()
        .spawn((
            (Text::new("Sample Text")),
            ForegroundRole(ColorRole::OnPrimary),
            TypeStyleToken(TypeScale::BodyLarge),
        ))
        .id();

    app.world_mut().entity_mut(entity).add_child(text_child);

    // First tick sets initial values
    app.update();

    let initial_bg = app.world().get::<BackgroundColor>(entity).copied();
    assert!(
        initial_bg.is_some(),
        "BackgroundColor should have been inserted"
    );

    // Now mutate theme from Dark to Light
    {
        let mut theme = app.world_mut().resource_mut::<MaterialTheme>();
        theme.scheme = ColorScheme::light();
    }

    // Second tick updates components
    app.update();

    let updated_bg = app.world().get::<BackgroundColor>(entity).copied();
    assert!(updated_bg.is_some());
    assert_ne!(
        initial_bg.map(|b| b.0),
        updated_bg.map(|b| b.0),
        "BackgroundColor must update when MaterialTheme changes"
    );
}

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<MaterialTheme>()
        .add_systems(Update, (sync_typestyle_tokens, sync_elevation_shadows));
    app
}

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

#[test]
fn raised_entities_get_a_shadow_and_flat_ones_lose_it() {
    let mut app = app();
    let e = app
        .world_mut()
        .spawn((Node::default(), ElevationToken(ElevationLevel::Level3)))
        .id();
    app.update();
    assert!(app.world().get::<BoxShadow>(e).is_some());
    app.world_mut().get_mut::<ElevationToken>(e).expect("t").0 = ElevationLevel::Level0;
    app.update();
    assert!(app.world().get::<BoxShadow>(e).is_none());
}
