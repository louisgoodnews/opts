//! Control-protocol methods of the viewpoint editor: `camera.viewpoint.get`, `.set`, `.save`,
//! `.reset`, `.export` and `.import`. They work on the same [`ViewpointState`] as the keys and the panel, so a scripted run
//! sees exactly what a player gets.


use bevy::prelude::*;
use opts_bevy::control::state::{ControlRegistry, Outcome};
use serde_json::Value;

mod save;
mod set;
mod transfer;
mod view;

use save::{reset, save};
use set::set;
use transfer::{export, import};
use view::describe;

/// Registers the methods.
pub fn register(registry: &mut ControlRegistry) {
    registry.register("camera.viewpoint.get", get);
    registry.register("camera.viewpoint.set", set);
    registry.register("camera.viewpoint.save", save);
    registry.register("camera.viewpoint.reset", reset);
    registry.register("camera.viewpoint.export", export);
    registry.register("camera.viewpoint.import", import);
}

fn get(world: &mut World, _: &Value) -> Outcome {
    Outcome::ok(describe(world))
}
