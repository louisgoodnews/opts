//! Game-specific methods of the control protocol: `state`, `scene`, `scene.load_map`,
//! `scene.unload`, `vehicle.set`, `camera.teleport`, `render.look`. The generic ones live in
//! `opts_bevy::control`; see `docs/src/architecture/control-protocol.md`.

use std::path::PathBuf;

use bevy::prelude::*;
use opts_bevy::control::handlers::query::base_state;
use opts_bevy::control::params::{f64_param, opt_bool, opt_f64, opt_str, str_param};
use opts_bevy::control::state::{ControlRegistry, Outcome, Waiter};
use opts_bevy::control::{channel::dispatch, PathPolicy};
use opts_bevy::coords::omsi_to_bevy_pos;
use opts_bevy::CameraState;
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

use crate::app::AppArgs;
use crate::app_state::AppState;
use crate::in_game::omsi_world::{WorldLoadJob, WorldStats};
use crate::in_game::types::{InGameSessionInfo, SimulationLoadState};

/// Registers the game's control methods (needs `opts_bevy::control::ControlPlugin` first).
pub struct GameControlPlugin;

impl Plugin for GameControlPlugin {
    fn build(&self, app: &mut App) {
        let Some(mut registry) = app.world_mut().get_resource_mut::<ControlRegistry>() else {
            return;
        };
        registry.register("state", state);
        registry.register("scene", scene);
        registry.register("scene.load_map", crate::control_loading::load_map);
        registry.register("scene.unload", unload);
        registry.register("vehicle.set", set_vehicle);
        registry.register("vehicle.script_vars", script_vars);
        registry.register("vehicle.get", vehicle_get);
        registry.register("camera.teleport", teleport);
        registry.register("render.look", render_look);
        registry.register("render.stats", render_stats);
        registry.register("ui.navigate", ui_navigate);
        registry.register("menu.do", menu_do);
        registry.register("menu.state", menu_state);
        crate::hud::protocol::register(&mut registry);
        crate::passengers::register(&mut registry);
        crate::language_packs::register(&mut registry);
        crate::control_ui::register(&mut registry);
        crate::control_drive::register(&mut registry);
        crate::viewpoints::protocol::register(&mut registry);
        crate::control_session::register(&mut registry);
        crate::control_environment::register(&mut registry);
        crate::formation::register(&mut registry);
        crate::control_save::register(&mut registry);
        crate::control_trace::register(&mut registry);
        crate::control_tiles::register(&mut registry);
        crate::control_audio::register(&mut registry);
        crate::control_hotspots::register(&mut registry);
        crate::control_textures::register(&mut registry);
        crate::control_loading::register(&mut registry);
        crate::control_setvars::register(&mut registry);
        crate::control_configs::register(&mut registry);
        crate::control_dynamics::register(&mut registry);
        crate::control_route::register(&mut registry);
        crate::control_scheduled::register(&mut registry);
        crate::control_rail::register(&mut registry);
        crate::control_profiles::register(&mut registry);
        crate::control_trace::add_overlay(app);
        app.add_systems(Update, allow_session_reads);
        if let Some(mut hooks) = app
            .world_mut()
            .get_resource_mut::<opts_bevy::capture::CaptureMetaHooks>()
        {
            hooks.0.push(capture_metadata);
        }
    }
}

/// Adds the game's view of the world to the metadata of every capture: application state, map,
/// vehicle and the tile the player (or, without a vehicle, the camera) is on.
fn capture_metadata(world: &World, m: &mut serde_json::Map<String, Value>) {
    m.insert("app_state".into(), json!(format!("{:?}", app_state(world))));
    if let Some(stats) = world.get_resource::<WorldStats>() {
        if !stats.map_name.is_empty() {
            m.insert("map".into(), json!(stats.map_name));
        }
        if !stats.vehicle_name.is_empty() {
            m.insert("vehicle".into(), json!(stats.vehicle_name));
        }
        let tile = stats
            .position
            .vehicle
            .map(|(p, _)| p.tile)
            .or(stats.position.camera.map(|p| p.tile));
        if let Some((x, y)) = tile {
            m.insert("tile".into(), json!([x, y]));
        }
    }
}

/// Lets control commands read below the OMSI installation: the one known at startup (so
/// `scene.load_map` works from the main menu) and the one of the running session.
fn allow_session_reads(
    job: Option<Res<WorldLoadJob>>,
    policy: Option<ResMut<PathPolicy>>,
    mut startup_done: Local<bool>,
) {
    let Some(mut policy) = policy else {
        return;
    };
    if !*startup_done {
        *startup_done = true;
        if let Some(root) = crate::omsi_root::known_omsi_root() {
            if !policy.read_roots.contains(&root) {
                policy.read_roots.push(root);
            }
        }
    }
    let Some(job) = job else {
        return;
    };
    let root = job.omsi_root().to_path_buf();
    if job.is_added() && !policy.read_roots.contains(&root) {
        policy.read_roots.push(root);
    }
}

fn app_state(world: &World) -> AppState {
    *world.resource::<State<AppState>>().get()
}

fn state(world: &mut World, _: &Value) -> Outcome {
    let mut m = base_state(world);
    m.insert("app_state".into(), json!(format!("{:?}", app_state(world))));
    if let Some(stats) = world.get_resource::<WorldStats>() {
        m.insert("map".into(), json!(stats.map_name));
        m.insert("vehicle".into(), json!(stats.vehicle_name));
    }
    if let Some(load) = world.get_resource::<SimulationLoadState>() {
        m.insert("loading".into(), json!({ "stage": format!("{:?}", load.stage), "progress": load.progress, "ready": load.hud_spawned }));
    }
    Outcome::ok(Value::Object(m))
}


mod scene;
mod vehicle;


pub(crate) use scene::restart_waiter;
use scene::*;
use vehicle::*;


mod handlers;
use handlers::{
    menu_do, menu_state, render_look, render_stats, script_vars, ui_navigate, vehicle_get,
};

#[cfg(test)]
mod restart_waiter_tests;
#[cfg(test)]
mod tests;
