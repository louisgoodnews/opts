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

/// A minimal app with the whole GUI plugin and the resources widgets read.
pub fn headless_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::input::InputPlugin, OptsGuiPlugin));
    app.update();
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

/// A full click on `entity`: press, release, one frame.
pub fn click(app: &mut App, entity: Entity) {
    press(app.world_mut(), entity);
    release(app.world_mut(), entity);
    app.update();
}
