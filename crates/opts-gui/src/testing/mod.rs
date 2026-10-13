//! Helpers for headless widget tests (no window, no renderer, no OMSI assets).
//!
//! Tests of the library, the game screens and the control protocol share them: a minimal app with
//! [`OptsGuiPlugin`], and synthetic pointer events that travel the same
//! observers a real mouse does.

use bevy::camera::NormalizedRenderTarget;
use bevy::picking::backend::HitData;
use bevy::picking::events::{Pointer, PointerPress, PointerRelease};
use bevy::picking::pointer::{Location, PointerButton, PointerId};
use bevy::prelude::*;

use crate::OptsGuiPlugin;

pub mod golden;
pub mod rules;

/// A minimal app with the whole GUI plugin and the resources widgets read.
pub fn headless_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::input::InputPlugin, OptsGuiPlugin));
    app.update();
    app
}

/// Like [`headless_app`] but with Bevy's real UI layout (no renderer, no window system): nodes get
/// a `ComputedNode`, so sizes and positions are the ones a player would see in a 1280x720 window.
pub fn layout_app() -> App {
    let mut app = layout_base();
    app.update();
    app
}

/// [`layout_app`] before its first frame: more plugins with `Startup` systems can still be added
/// (a `Startup` system added after the first update never runs).
pub fn layout_base() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::window::WindowPlugin::default(),
        bevy::input::InputPlugin,
        bevy::transform::TransformPlugin,
        bevy::image::ImagePlugin::default(),
        bevy::text::TextPlugin,
        bevy::camera::CameraPlugin,
        bevy::a11y::AccessibilityPlugin,
        bevy::picking::DefaultPickingPlugins,
        bevy::ui::UiPlugin,
    ));
    app.init_asset::<Mesh>()
        .init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>()
        .init_asset::<bevy::image::TextureAtlasLayout>();
    app.add_plugins(OptsGuiPlugin);
    app.world_mut().spawn(Camera2d);
    app
}

fn pointer() -> Pointer {
    Pointer::new(
        PointerId::Mouse,
        Location {
            target: NormalizedRenderTarget::None {
                width: 1280,
                height: 720,
            },
            position: Vec2::ZERO,
        },
    )
}

fn hit() -> HitData {
    HitData::new(Entity::PLACEHOLDER, 0.0, None, None)
}

/// Presses the primary pointer button on `entity` (bubbles to its ancestors like a real press).
pub fn press(world: &mut World, entity: Entity) {
    world.trigger(PointerPress {
        entity,
        pointer: pointer(),
        button: PointerButton::Primary,
        hit: hit(),
        count: 1,
    });
    world.flush();
}

/// Releases the primary pointer button over `entity`.
pub fn release(world: &mut World, entity: Entity) {
    world.trigger(PointerRelease {
        entity,
        pointer: pointer(),
        button: PointerButton::Primary,
        hit: hit(),
    });
    world.flush();
}

/// Turns the mouse wheel by `lines` lines over `entity` (positive scrolls the content up, like a
/// real wheel turned away from the user); the event bubbles like a real one.
pub fn wheel(world: &mut World, entity: Entity, lines: f32) {
    world.trigger(bevy::picking::events::PointerScroll {
        entity,
        pointer: pointer(),
        unit: bevy::input::mouse::MouseScrollUnit::Line,
        x: 0.0,
        y: -lines,
        hit: hit(),
        phase: bevy::input::touch::TouchPhase::Moved,
    });
    world.flush();
}

/// A full click on `entity`: press, release, one frame.
pub fn click(app: &mut App, entity: Entity) {
    press(app.world_mut(), entity);
    release(app.world_mut(), entity);
    app.update();
}

/// Sends one [`UiAction`](crate::actions::UiAction) and runs two frames (action, then reaction).
pub fn act(app: &mut App, action: crate::actions::UiAction) {
    app.world_mut().write_message(action);
    app.update();
    app.update();
}

/// The entity that currently carries [`Focused`](crate::interaction::state::Focused).
pub fn focused(app: &mut App) -> Option<Entity> {
    app.world_mut()
        .query_filtered::<Entity, With<crate::interaction::state::Focused>>()
        .iter(app.world())
        .next()
}

/// Spawns a column of `n` filled buttons labelled `B0..`, returns their entities.
pub fn button_column(app: &mut App, n: usize) -> Vec<Entity> {
    let ids = {
        let mut commands = app.world_mut().commands();
        let root = commands
            .spawn(Node {
                flex_direction: FlexDirection::Column,
                ..default()
            })
            .id();
        let ids: Vec<Entity> = (0..n)
            .map(|i| {
                crate::components::button::ButtonBuilder::filled(format!("B{i}"))
                    .spawn(&mut commands)
            })
            .collect();
        commands.entity(root).add_children(&ids);
        ids
    };
    app.update();
    ids
}
