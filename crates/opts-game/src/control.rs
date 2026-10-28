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

fn scene(world: &mut World, _: &Value) -> Outcome {
    let Some(stats) = world.get_resource::<WorldStats>().cloned() else {
        return Outcome::err(RpcError::new(
            code::NOT_READY,
            "no session has been started",
        ));
    };
    let meshes = world.resource::<Assets<Mesh>>().len();
    let materials = world.resource::<Assets<StandardMaterial>>().len();
    let images = world.resource::<Assets<Image>>().len();
    let live_entities = world.query::<Entity>().iter(world).count();
    Outcome::ok(json!({
        "map": stats.map_name,
        "start_tile": [stats.start_tile.0, stats.start_tile.1],
        "tiles_loaded": stats.tiles_loaded,
        "tiles_missing": stats.tiles_missing,
        "entity_count": world.entities().len(),
        "entities_live": live_entities,
        "meshes": meshes,
        "materials": materials,
        "images": images,
        "asset_count": meshes + materials + images,
        "stats": {
            "objects_loaded": stats.objects_loaded,
            "objects_failed": stats.objects_failed,
            "objects_protected": stats.objects_protected,
            "trees": stats.trees_loaded,
            "splines_loaded": stats.splines_loaded,
            "splines_failed": stats.splines_failed,
            "scene_vertices": stats.scene_vertices,
            "asset_failures": stats.asset_failures,
            "vehicle": stats.vehicle_name,
        },
    }))
}

/// A waiter that drives the app through leaving the current session and entering a fresh one; it
/// finishes with a summary once the new session's HUD is up.
pub(crate) fn restart_waiter(world: &mut World) -> Waiter {
    let was_in_game = app_state(world) == AppState::InGame;
    world
        .resource_mut::<NextState<AppState>>()
        .set(if was_in_game {
            AppState::MainMenu
        } else {
            AppState::InGame
        });
    let mut left = !was_in_game;
    Box::new(move |world| {
        if !left {
            if app_state(world) != AppState::MainMenu {
                return None;
            }
            left = true;
            world
                .resource_mut::<NextState<AppState>>()
                .set(AppState::InGame);
            return None;
        }
        if app_state(world) != AppState::InGame {
            return None;
        }
        // A load that failed answers at once instead of timing out; a load with problems answers
        // while the loading screen waits for *Continue* (the session is usable, `loading.continue`
        // starts it), so a script is never blocked on a person's click.
        if let Some(report) = world.get_resource::<crate::in_game::load_report::LoadReport>() {
            use crate::in_game::load_report::LoadOutcome;
            let details: Vec<String> = report.problems.iter().map(|p| p.detail.clone()).collect();
            match report.outcome {
                LoadOutcome::Failed => {
                    return Some(Err(RpcError::new(
                        code::FAILED,
                        format!("the load failed: {}", details.join("; ")),
                    )));
                }
                LoadOutcome::ReadyWithProblems => {
                    if crate::control_loading::accepted_and_starting(world, report) {
                        return None;
                    }
                    let stats = world.resource::<WorldStats>();
                    return Some(Ok(json!({
                        "map": stats.map_name, "vehicle": stats.vehicle_name,
                        "tiles_loaded": stats.tiles_loaded,
                        "waiting_for_continue": !world
                            .get_resource::<SimulationLoadState>()
                            .is_some_and(|s| s.hud_spawned),
                        "problems": details,
                    })));
                }
                _ => {}
            }
        }
        let ready = world
            .get_resource::<SimulationLoadState>()
            .is_some_and(|s| s.hud_spawned);
        ready.then(|| {
            let stats = world.resource::<WorldStats>();
            Ok(json!({ "map": stats.map_name, "vehicle": stats.vehicle_name, "tiles_loaded": stats.tiles_loaded }))
        })
    })
}

/// Brings the app into a fresh in-game session, leaving the current one first. Answers when the
/// HUD of the new session is up.
fn restart_session(world: &mut World) -> Outcome {
    Outcome::Wait(restart_waiter(world))
}

fn read_path(world: &World, params: &Value, name: &str) -> Result<PathBuf, RpcError> {
    let raw = str_param(params, name)?;
    world
        .resource::<PathPolicy>()
        .check_read(&PathBuf::from(raw))
}

fn unload(world: &mut World, _: &Value) -> Outcome {
    world
        .resource_mut::<NextState<AppState>>()
        .set(AppState::MainMenu);
    Outcome::ok(json!({ "ok": true }))
}

fn set_vehicle(world: &mut World, params: &Value) -> Outcome {
    let file = match read_path(world, params, "path") {
        Ok(p) if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("bus")) => p,
        Ok(p) => {
            return Outcome::err(RpcError::invalid_params(format!(
                "{} is not a .bus file",
                p.display()
            )))
        }
        Err(e) => return Outcome::err(e),
    };
    world.resource_mut::<AppArgs>().0.vehicle_path = Some(file);
    if world.get_resource::<InGameSessionInfo>().is_none() {
        return Outcome::ok(json!({ "ok": true, "applied": "at the next session start" }));
    }
    restart_session(world)
}

/// OMSI world coordinates (metres, absolute) and compass heading → free-roam camera.
fn teleport(world: &mut World, params: &Value) -> Outcome {
    let parsed = (|| -> Result<(f64, f64, f64, f64, f64), RpcError> {
        Ok((
            f64_param(params, "x")?,
            f64_param(params, "y")?,
            f64_param(params, "z")?,
            opt_f64(params, "yaw")?.unwrap_or(0.0),
            opt_f64(params, "pitch")?.unwrap_or(0.0),
        ))
    })();
    let (x, y, z, yaw, pitch) = match parsed {
        Ok(v) => v,
        Err(e) => return Outcome::err(e),
    };
    let Some((start, grid)) = world
        .get_resource::<WorldStats>()
        .map(|s| (s.start_tile, s.grid))
    else {
        return Outcome::err(RpcError::new(
            code::NOT_READY,
            "no session has been started",
        ));
    };
    let local = grid.absolute_to_local(bevy::math::DVec2::new(x, y), start);
    let local = Vec3::new(local.x as f32, local.y as f32, z as f32);
    let position = omsi_to_bevy_pos(local);
    let (yaw_rad, pitch_rad) = ((-yaw).to_radians() as f32, pitch.to_radians() as f32);
    let switched = dispatch(world, "camera.set_mode", &json!({ "mode": "free" }));
    let Outcome::Wait(mut switch) = switched else {
        return switched;
    };
    Outcome::Wait(Box::new(move |world| {
        let result = switch(world)?;
        if let Err(e) = result {
            return Some(Err(e));
        }
        let rotation = Quat::from_euler(EulerRot::YXZ, yaw_rad, pitch_rad, 0.0);
        let mut cameras = world.query_filtered::<&mut Transform, With<opts_bevy::MainCamera>>();
        for mut t in cameras.iter_mut(world) {
            t.translation = position;
            t.rotation = rotation;
        }
        let mut cam = world.resource_mut::<CameraState>();
        cam.free_roam_yaw = yaw_rad;
        cam.free_roam_pitch = pitch_rad;
        Some(Ok(
            json!({ "translation": [position.x, position.y, position.z] }),
        ))
    }))
}

mod handlers;
use handlers::{
    menu_do, menu_state, render_look, render_stats, script_vars, ui_navigate, vehicle_get,
};

#[cfg(test)]
mod restart_waiter_tests;
#[cfg(test)]
mod tests;
