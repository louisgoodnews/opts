//! Unit tests for `super` (`layer.rs`).

use super::*;
use crate::components::button::ButtonBuilder;
use crate::testing::headless_app;

fn spawn(app: &mut App) -> (Entity, StateLayerLink) {
    let entity = {
        let mut commands = app.world_mut().commands();
        ButtonBuilder::filled("Go").spawn(&mut commands)
    };
    app.update();
    app.update();
    let link = *app.world().get::<StateLayerLink>(entity).expect("link");
    (entity, link)
}

fn alpha(app: &App, e: Entity) -> f32 {
    app.world()
        .get::<BackgroundColor>(e)
        .map_or(-1.0, |b| b.0.alpha())
}

#[test]
fn rest_hover_focus_pressed_have_the_documented_layers() {
    let mut app = headless_app();
    let (button, link) = spawn(&mut app);
    assert_eq!(alpha(&app, link.layer), 0.0);

    app.world_mut().entity_mut(button).insert(Hovered(true));
    app.update();
    assert!((alpha(&app, link.layer) - 0.08).abs() < 1e-4, "hover 8 %");

    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    app.world_mut().entity_mut(button).insert(Pressed);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .release(MouseButton::Left);
    assert!(
        (alpha(&app, link.layer) - 0.10).abs() < 1e-4,
        "pressed 10 %"
    );

    app.world_mut().entity_mut(button).remove::<Pressed>();
    app.world_mut().entity_mut(button).insert(Hovered(false));
    app.world_mut().entity_mut(button).insert(Focused);
    app.update();
    assert!((alpha(&app, link.layer) - 0.10).abs() < 1e-4, "focus 10 %");
    assert!(app.world().get::<Outline>(button).is_some(), "focus ring");

    app.world_mut().entity_mut(button).remove::<Focused>();
    app.update();
    assert!(app.world().get::<Outline>(button).is_none());
    assert_eq!(alpha(&app, link.layer), 0.0);
}

#[test]
fn disabled_scrim_leaves_38_percent_and_blocks_hover() {
    let mut app = headless_app();
    let (button, link) = spawn(&mut app);
    assert_eq!(alpha(&app, link.scrim), 0.0);
    app.world_mut()
        .entity_mut(button)
        .insert((Disabled, Hovered(true)));
    app.update();
    assert!((alpha(&app, link.scrim) - 0.62).abs() < 1e-4);
    assert_eq!(alpha(&app, link.layer), 0.0, "no hover layer when disabled");
}

#[test]
fn focus_ring_is_hidden_after_pointer_focus() {
    let mut app = headless_app();
    let (button, _) = spawn(&mut app);
    app.world_mut().insert_resource(InputFocusVisible(false));
    app.world_mut().entity_mut(button).insert(Focused);
    app.update();
    assert!(app.world().get::<Outline>(button).is_none());
}

#[test]
fn error_state_swaps_the_border_and_restores_it() {
    let mut app = headless_app();
    let (button, _) = spawn(&mut app);
    app.world_mut()
        .entity_mut(button)
        .insert(BorderRole(ColorRole::Outline));
    app.update();
    app.world_mut()
        .entity_mut(button)
        .insert(ErrorState::new("Needs a name"));
    app.update();
    assert_eq!(
        app.world().get::<BorderRole>(button).map(|b| b.0),
        Some(ColorRole::Error)
    );
    app.world_mut().entity_mut(button).remove::<ErrorState>();
    app.update();
    assert_eq!(
        app.world().get::<BorderRole>(button).map(|b| b.0),
        Some(ColorRole::Outline)
    );
}
