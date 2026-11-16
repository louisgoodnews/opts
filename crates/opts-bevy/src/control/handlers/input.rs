//! `input.*`: injected input arrives as ordinary Bevy input events, so it takes exactly the path
//! of real keyboard and mouse input (including the modal gating).

mod keyboard;
mod mouse;

use bevy::ecs::entity::Entity;
use bevy::ecs::world::World;
use bevy::time::{Real, Time};
use bevy::window::PrimaryWindow;

use crate::control::state::ControlRegistry;

use keyboard::{key, key_hold, key_release, text};
use mouse::{mouse_click, mouse_move, mouse_scroll};

/// Default key press length in milliseconds.
const DEFAULT_PRESS_MS: u64 = 50;
/// Longest press a client may request.
const MAX_PRESS_MS: u64 = 600_000;

/// Registers the input methods.
pub fn register(r: &mut ControlRegistry) {
    r.register("input.key", key);
    r.register("input.key_hold", key_hold);
    r.register("input.key_release", key_release);
    r.register("input.text", text);
    r.register("input.mouse.move", mouse_move);
    r.register("input.mouse.click", mouse_click);
    r.register("input.mouse.scroll", mouse_scroll);
}

/// The primary window, or a placeholder when there is none (headless runs and tests); Bevy's
/// input systems do not look at the window of a keyboard event.
pub fn primary_window_or_placeholder(world: &mut World) -> Entity {
    let mut query = world.query_filtered::<Entity, bevy::ecs::query::With<PrimaryWindow>>();
    query.iter(world).next().unwrap_or(Entity::PLACEHOLDER)
}

fn now(world: &World) -> f64 {
    world.resource::<Time<Real>>().elapsed_secs_f64()
}
