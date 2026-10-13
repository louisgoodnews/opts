//! Game-specific methods of the control protocol: `state`, `scene`, `scene.load_map`,
//! `scene.unload`, `vehicle.set`, `camera.teleport`, `render.look`. The generic ones live in
//! `opts_bevy::control`; see `docs/src/architecture/control-protocol.md`.

use std::path::PathBuf;

use bevy::prelude::*;
use opts_bevy::control::handlers::query::base_state;
use opts_bevy::control::params::{f64_param, opt_f64, opt_str, str_param};
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
use crate::new_game_screen::types::NewGameSetup;

/// Registers the game's control methods (needs `opts_bevy::control::ControlPlugin` first).
pub struct GameControlPlugin;

impl Plugin for GameControlPlugin {
    fn build(&self, app: &mut App) {
        let Some(mut registry) = app.world_mut().get_resource_mut::<ControlRegistry>() else {
            return;
        };
        registry.register("state", state);
        registry.register("scene", scene);
        registry.register("scene.load_map", load_map);
        registry.register("scene.unload", unload);
        registry.register("vehicle.set", set_vehicle);
        registry.register("vehicle.script_vars", script_vars);
        registry.register("vehicle.get", vehicle_get);
        registry.register("camera.teleport", teleport);
        registry.register("render.look", render_look);
        registry.register("ui.navigate", ui_navigate);
        registry.register("menu.do", menu_do);
        registry.register("menu.state", menu_state);
        crate::hud::protocol::register(&mut registry);
        crate::language_packs::register(&mut registry);
        crate::control_ui::register(&mut registry);
        crate::control_drive::register(&mut registry);
        crate::viewpoints::protocol::register(&mut registry);
        crate::control_session::register(&mut registry);
        app.add_systems(Update, allow_session_reads);
    }
}

/// Lets control commands read below the OMSI installation of the running session.
fn allow_session_reads(job: Option<Res<WorldLoadJob>>, policy: Option<ResMut<PathPolicy>>) {
    let (Some(job), Some(mut policy)) = (job, policy) else {
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
    Outcome::ok(json!({
        "map": stats.map_name,
        "start_tile": [stats.start_tile.0, stats.start_tile.1],
        "tiles_loaded": stats.tiles_loaded,
        "tiles_missing": stats.tiles_missing,
        "entity_count": world.entities().len(),
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
        let ready = app_state(world) == AppState::InGame
            && world
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

fn load_map(world: &mut World, params: &Value) -> Outcome {
    let dir = match read_path(world, params, "path") {
        Ok(p) if p.is_dir() => p,
        Ok(p) => {
            return Outcome::err(RpcError::invalid_params(format!(
                "{} is not a map folder",
                p.display()
            )))
        }
        Err(e) => return Outcome::err(e),
    };
    let entry = match opt_str(params, "entry") {
        Ok(e) => e.map(str::to_string),
        Err(e) => return Outcome::err(e),
    };
    world.resource_mut::<AppArgs>().0.map_path = Some(dir);
    if let (Some(entry), Some(mut setup)) = (entry, world.get_resource_mut::<NewGameSetup>()) {
        setup.selected_entry_point = entry;
    }
    restart_session(world)
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
    let Some(start) = world.get_resource::<WorldStats>().map(|s| s.start_tile) else {
        return Outcome::err(RpcError::new(
            code::NOT_READY,
            "no session has been started",
        ));
    };
    let local = Vec3::new(
        x as f32 - start.0 as f32 * opts_bevy::tile_size(),
        y as f32 - start.1 as f32 * opts_bevy::tile_size(),
        z as f32,
    );
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
        let mut cameras = world.query_filtered::<&mut Transform, With<Camera3d>>();
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

/// `ui.navigate {screen}`: opens a shell destination or deep link (`settings`,
/// `screen:settings/graphics`) exactly like a rail click; the effect shows a frame later.
fn ui_navigate(world: &mut World, params: &Value) -> Outcome {
    let target = match str_param(params, "screen") {
        Ok(t) => t.to_string(),
        Err(e) => return Outcome::err(e),
    };
    // The rail is behind an open dialog for a person; the command is too.
    if !opts_bevy::control::handlers::ui_ext::open_modals(world).is_empty() {
        return Outcome::err(RpcError::new(
            code::GATED,
            "a dialog is open and blocks navigation (answer it first: `ui.screen` lists its buttons)",
        ));
    }
    let (id, _) = crate::shell::nav::parse_target(&target);
    let known = world
        .get_resource::<crate::shell::ScreenRegistry>()
        .is_some_and(|r| r.get(id).is_some());
    if !known {
        return Outcome::err(RpcError::new(code::NOT_FOUND, format!("no screen `{id}`")));
    }
    world.write_message(crate::shell::Navigate::to(target.clone()));
    Outcome::ok(json!({ "ok": true, "screen": target }))
}

/// `menu.do {command}`: drives the in-game vehicle menu (`MenuCommands`).
fn menu_do(world: &mut World, params: &Value) -> Outcome {
    let command = match str_param(params, "command") {
        Ok(c) => c.to_string(),
        Err(e) => return Outcome::err(e),
    };
    let Some(mut q) = world.get_resource_mut::<crate::in_game::MenuCommands>() else {
        return Outcome::err(RpcError::new(code::NOT_READY, "the menu is not running"));
    };
    q.0.push(command.clone());
    Outcome::ok(json!({ "queued": command }))
}

/// `menu.state`: page, choices and the free entry points of the vehicle menu.
fn menu_state(world: &mut World, _: &Value) -> Outcome {
    match world.get_resource::<crate::in_game::VehicleMenu>() {
        Some(m) => Outcome::ok(m.describe()),
        None => Outcome::err(RpcError::new(code::NOT_READY, "the menu is not running")),
    }
}

/// `vehicle.script_vars {contains?}`: the player vehicle's script variables (name → value),
/// optionally filtered by a case-insensitive substring.
fn script_vars(world: &mut World, params: &Value) -> Outcome {
    let needle = params
        .get("contains")
        .and_then(Value::as_str)
        .map(str::to_lowercase);
    let mut query =
        world.query_filtered::<&opts_bevy::VehicleScripts, With<opts_bevy::PlayerVehicle>>();
    let Some(scripts) = query.iter(world).next() else {
        return Outcome::err(RpcError::new(
            code::NOT_READY,
            "the player vehicle has no scripts",
        ));
    };
    let Ok(engine) = scripts.engine.lock() else {
        return Outcome::err(RpcError::new(code::NOT_READY, "script engine unavailable"));
    };
    let vars: serde_json::Map<String, Value> = engine
        .variables()
        .into_iter()
        .filter(|(n, _)| needle.as_deref().is_none_or(|s| n.contains(s)))
        .map(|(n, v)| (n, json!(v)))
        .collect();
    Outcome::ok(
        json!({ "count": vars.len(), "variables": vars, "ops_executed": engine.ops_executed() }),
    )
}

/// `vehicle.get`: the player vehicle's absolute OMSI position, heading and speed.
fn vehicle_get(world: &mut World, _: &Value) -> Outcome {
    let mut query =
        world.query_filtered::<&opts_bevy::VehicleController, With<opts_bevy::PlayerVehicle>>();
    let Some(c) = query.iter(world).next() else {
        return Outcome::err(RpcError::new(code::NOT_READY, "there is no player vehicle"));
    };
    let ch = &c.sim.chassis;
    // The chassis position is local to the start tile; the protocol speaks absolute coordinates
    // (the same as `camera.teleport`).
    let (tx, ty) = world
        .get_resource::<WorldStats>()
        .map_or((0, 0), |s| s.start_tile);
    Outcome::ok(json!({
        "x": f64::from(ch.position.x) + f64::from(tx) * f64::from(opts_bevy::tile_size()),
        "y": f64::from(ch.position.y) + f64::from(ty) * f64::from(opts_bevy::tile_size()),
        "z": ch.elevation_z,
        "heading_deg": ch.heading_rad.to_degrees(),
        "speed_kmh": ch.forward_speed_km_h(),
    }))
}

/// `render.look {look?}`: reads the colour look, or sets it (`vivid`, `natural`, `bevy`). A set
/// takes effect at once on every camera and the sky, and is saved in the settings.
fn render_look(world: &mut World, params: &Value) -> Outcome {
    let current = world.resource::<crate::look::ActiveLook>().0;
    let requested = match opt_str(params, "look") {
        Ok(r) => r,
        Err(e) => return Outcome::err(e),
    };
    let look = match requested {
        None => current,
        Some(text) => match crate::look::Look::parse(text) {
            Some(l) => l,
            None => {
                return Outcome::err(RpcError::invalid_params(format!(
                    "unknown look `{text}` (vivid, natural, bevy)"
                )))
            }
        },
    };
    if requested.is_some() {
        world.resource_mut::<crate::look::ActiveLook>().0 = look;
        if let Some(mut settings) = world.get_resource_mut::<crate::settings::GameSettings>() {
            settings.look = look;
            settings.save_in_background();
        }
    }
    let available: Vec<&str> = crate::look::Look::ALL.iter().map(|l| l.key()).collect();
    Outcome::ok(json!({ "look": look.key(), "previous": current.key(), "available": available }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::look::{ActiveLook, Look};

    fn answer(outcome: Outcome) -> Result<Value, RpcError> {
        match outcome {
            Outcome::Done(r) => r,
            Outcome::Wait(_) => panic!("render.look answers immediately"),
        }
    }

    #[test]
    fn render_look_gets_sets_and_rejects() {
        let mut world = World::new();
        world.insert_resource(ActiveLook::default());
        // get: Vivid is the default.
        let got = answer(render_look(&mut world, &json!({}))).expect("get");
        assert_eq!(got["look"], "vivid");
        assert_eq!(got["available"], json!(["vivid", "natural", "bevy"]));
        // set: takes effect on the resource.
        let set = answer(render_look(&mut world, &json!({ "look": "bevy" }))).expect("set");
        assert_eq!(
            (set["look"].as_str(), set["previous"].as_str()),
            (Some("bevy"), Some("vivid"))
        );
        assert_eq!(world.resource::<ActiveLook>().0, Look::BevyDefault);
        // unknown names are invalid parameters and change nothing.
        let err = answer(render_look(&mut world, &json!({ "look": "sepia" }))).expect_err("bad");
        assert_eq!(err.code, opts_cli::protocol::code::INVALID_PARAMS);
        assert_eq!(world.resource::<ActiveLook>().0, Look::BevyDefault);
    }
}
