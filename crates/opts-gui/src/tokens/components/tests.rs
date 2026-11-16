//! Unit tests for `super` (`components.rs`).

use super::*;
use crate::theme::theme::MaterialTheme;
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

mod type_style;

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
