//! The sections this build saves. Each one is registered like any feature's would be; the save
//! system (`container`, `store`, `registry`, `ops`, `restore`) does not know them.
//!
//! | id | holds | restored |
//! | :-- | :-- | :-- |
//! | `session` | map folder and name, start point, mode, play time | before the session starts |
//! | `time` | date, clock base, simulated seconds since the start | before; clock when the world is up |
//! | `weather` | weather preset | before |
//! | `session_vehicles` | the registry with every vehicle's live state | vehicles rebuilt by the spawner |
//! | `active_vehicle` | which vehicle the driver is in | when all vehicles exist |
//! | `camera` | mode, sub-camera, free camera pose, orbit and look offsets | when all vehicles exist |
//! | `counters` | driven distance, session duration and top speed | when all vehicles exist |
//! | `simulation` | paused, time scale | last |

use std::path::Path;

use bevy::prelude::*;
use opts_bevy::vehicle_remote::RemoteControls;
use opts_bevy::{CameraAvailability, CameraMode, CameraState};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::registry::{
    CheckEnv, Issue, IssueKind, SaveAppExt, SaveSection, Stage,
};
use super::{fingerprint, PlayClock};
use crate::in_game::omsi_world::WorldLoadJob;
use crate::in_game::types::InGameSessionInfo;
use crate::new_game_screen::types::NewGameSetup;
use crate::pause::{Pause, SessionStats};
use crate::session_vehicles::{ops, EntrySel, SessionVehicles, SAVE_SECTION, SAVE_SECTION_VERSION};

fn parse<T: for<'de> Deserialize<'de>>(data: &Value, what: &str) -> Result<T, String> {
    serde_json::from_value(data.clone()).map_err(|e| format!("{what}: {e}"))
}

fn to_value<T: Serialize>(value: &T) -> Result<Option<Value>, String> {
    serde_json::to_value(value)
        .map(Some)
        .map_err(|e| e.to_string())
}

/// Where a saved vehicle's `.bus` file is now: the saved path if it exists, else the path relative
/// to the installation `root`.
#[must_use]
pub fn resolve_bus_file(saved: &str, rel: Option<&str>, root: Option<&Path>) -> Option<std::path::PathBuf> {
    let direct = std::path::PathBuf::from(saved);
    if direct.is_file() {
        return Some(direct);
    }
    let found = root?.join(rel?);
    found.is_file().then_some(found)
}

// ---- session ---------------------------------------------------------------------------------

/// Data of the `session` section.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionData {
    /// The map's folder below `<OMSI>/maps`.
    pub map_folder: String,
    /// The map's display name.
    pub map_name: String,
    /// The start point chosen for the drive (the drive's entry point name).
    pub entry_point: String,
    /// What kind of session this is (`drive`; later modes get other words).
    pub mode: String,
    /// Seconds of play, summed over every load of this save.
    pub played_s: f64,
}

fn session_capture(world: &mut World) -> Result<Option<Value>, String> {
    let job = world
        .get_resource::<WorldLoadJob>()
        .ok_or("no map is loaded")?;
    let map_folder = job
        .config
        .map_dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .ok_or("the map folder has no name")?;
    let info = world.resource::<InGameSessionInfo>();
    to_value(&SessionData {
        map_folder,
        map_name: info.map_name.clone(),
        entry_point: info.entry_point.clone(),
        mode: "drive".to_owned(),
        played_s: world.get_resource::<PlayClock>().map_or(0.0, |c| c.played_s),
    })
}

fn session_prepare(world: &mut World, data: &Value) -> Result<(), String> {
    let d: SessionData = parse(data, "session")?;
    let mut setup = world.resource_mut::<NewGameSetup>();
    setup.selected_map = d.map_folder;
    setup.selected_entry_point = d.entry_point;
    // The vehicles come back through the spawner, not the loading pipeline's first vehicle.
    setup.no_vehicle = true;
    setup.selected_vehicle.clear();
    setup.selected_livery.clear();
    setup.custom_setvars.clear();
    world.resource_mut::<PlayClock>().played_s = d.played_s;
    Ok(())
}

fn session_check(data: &Value, env: &CheckEnv) -> Vec<Issue> {
    let Ok(d) = parse::<SessionData>(data, "session") else {
        return Vec::new();
    };
    let missing = |blocking| Issue {
        kind: IssueKind::MissingMap,
        subject: d.map_name.clone(),
        blocking,
    };
    let Some(root) = env.root else {
        return vec![missing(true)];
    };
    let dir = root.join("maps").join(&d.map_folder);
    if !dir.is_dir() {
        return vec![missing(true)];
    }
    match (&env.fingerprint.map, fingerprint::map(&dir)) {
        (Some((_, saved)), Some(now)) if *saved != now => vec![Issue {
            kind: IssueKind::Changed,
            subject: d.map_name,
            blocking: false,
        }],
        _ => Vec::new(),
    }
}

// ---- time ------------------------------------------------------------------------------------

/// Data of the `time` section.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeData {
    /// Calendar year of the session's start.
    pub year: i32,
    /// Month 1-12.
    pub month: u32,
    /// Day 1-31.
    pub day: u32,
    /// Hour of the clock when the session started.
    pub start_hour: u32,
    /// Minute of the clock when the session started.
    pub start_minute: u32,
    /// Simulated seconds since then (the clock shows start + this).
    pub sim_elapsed_s: f64,
}

fn time_capture(world: &mut World) -> Result<Option<Value>, String> {
    let setup = world.resource::<NewGameSetup>();
    let info = world.resource::<InGameSessionInfo>();
    to_value(&TimeData {
        year: setup.year,
        month: setup.month,
        day: setup.day,
        start_hour: info.start_hour,
        start_minute: info.start_minute,
        sim_elapsed_s: f64::from(info.sim_elapsed_seconds),
    })
}

fn time_prepare(world: &mut World, data: &Value) -> Result<(), String> {
    let d: TimeData = parse(data, "time")?;
    let mut setup = world.resource_mut::<NewGameSetup>();
    setup.year = d.year;
    setup.month = d.month;
    setup.day = d.day;
    setup.hour = d.start_hour;
    setup.minute = d.start_minute;
    Ok(())
}

fn time_apply(world: &mut World, data: &Value) -> Result<(), String> {
    let d: TimeData = parse(data, "time")?;
    world.resource_mut::<InGameSessionInfo>().sim_elapsed_seconds = d.sim_elapsed_s as f32;
    Ok(())
}

// ---- weather ---------------------------------------------------------------------------------

fn weather_capture(world: &mut World) -> Result<Option<Value>, String> {
    Ok(Some(json!({ "preset": world.resource::<NewGameSetup>().weather_preset })))
}

fn weather_prepare(world: &mut World, data: &Value) -> Result<(), String> {
    let preset = data
        .get("preset")
        .and_then(Value::as_str)
        .ok_or("weather: `preset` is missing")?;
    world.resource_mut::<NewGameSetup>().weather_preset = preset.to_owned();
    Ok(())
}

// ---- session vehicles ------------------------------------------------------------------------

fn root_of(world: &World) -> Option<std::path::PathBuf> {
    world
        .get_resource::<WorldLoadJob>()
        .map(|j| j.config.root.clone())
        .or_else(crate::omsi_root::known_omsi_root)
}

fn vehicles_capture(world: &mut World) -> Result<Option<Value>, String> {
    let mut reg = world.resource::<SessionVehicles>().clone();
    let root = root_of(world);
    let ids: Vec<_> = reg
        .records()
        .iter()
        .filter(|r| r.is_alive())
        .map(|r| (r.id, r.entity))
        .collect();
    for (id, entity) in ids {
        let state = entity.and_then(|e| crate::session_vehicles::state::capture(world, e));
        if let Some(r) = reg.get_mut(id) {
            r.saved_state = state;
        }
    }
    let rels: Vec<_> = reg
        .records()
        .iter()
        .map(|r| {
            (
                r.id,
                root.as_deref().and_then(|root| {
                    Path::new(&r.bus_file)
                        .strip_prefix(root)
                        .ok()
                        .map(|p| p.to_string_lossy().replace('\\', "/"))
                }),
            )
        })
        .collect();
    for (id, rel) in rels {
        if let Some(r) = reg.get_mut(id) {
            r.bus_rel = rel;
        }
    }
    Ok(Some(reg.to_section()))
}

fn vehicles_migrate(from: u32, mut data: Value) -> Result<Value, String> {
    match from {
        // Version 2 added `saved_state` and `bus_rel` to the records; both default when absent.
        1 => {
            data["version"] = json!(SAVE_SECTION_VERSION);
            Ok(data)
        }
        other => Err(format!("no migration from version {other}")),
    }
}

fn vehicles_apply(world: &mut World, data: &Value) -> Result<(), String> {
    let mut reg = SessionVehicles::from_section(data)?;
    let root = root_of(world);
    let ids: Vec<_> = reg.records().iter().map(|r| r.id).collect();
    for id in ids {
        let Some(r) = reg.get_mut(id) else { continue };
        if let Some(path) = resolve_bus_file(&r.bus_file, r.bus_rel.as_deref(), root.as_deref()) {
            r.bus_file = path.display().to_string();
        }
        // The entry point it really stood at is the most exact request there is.
        if let Some(entry) = r.spawn_entry_id {
            r.entry = EntrySel::Id(entry);
        }
    }
    // The active vehicle is restored last (`active_vehicle`): building a vehicle activates it.
    reg.set_active(None);
    *world.resource_mut::<SessionVehicles>() = reg;
    Ok(())
}

fn vehicles_check(data: &Value, env: &CheckEnv) -> Vec<Issue> {
    let records = data
        .get("vehicles")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut issues = Vec::new();
    let mut missing = Vec::new();
    for r in &records {
        let text = |key| r.get(key).and_then(Value::as_str);
        let saved = text("bus_file").unwrap_or_default();
        let name = text("name").unwrap_or(saved).to_owned();
        match resolve_bus_file(saved, text("bus_rel"), env.root) {
            None => missing.push(name),
            Some(path) => {
                let key = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                if let (Some(old), Some(now)) =
                    (env.fingerprint.vehicles.get(&key), fingerprint::vehicle(&path))
                {
                    if *old != now {
                        issues.push(Issue {
                            kind: IssueKind::Changed,
                            subject: name,
                            blocking: false,
                        });
                    }
                }
            }
        }
    }
    // Missing vehicles block the load only when none of the saved vehicles can be rebuilt.
    let all_missing = !records.is_empty() && missing.len() == records.len();
    issues.extend(missing.into_iter().map(|subject| Issue {
        kind: IssueKind::MissingVehicle,
        subject,
        blocking: all_missing,
    }));
    issues
}

// ---- active vehicle --------------------------------------------------------------------------

fn active_capture(world: &mut World) -> Result<Option<Value>, String> {
    let reg = world.resource::<SessionVehicles>();
    Ok(Some(json!({ "id": reg.active_id() })))
}

fn active_apply(world: &mut World, data: &Value) -> Result<(), String> {
    let Some(id) = data
        .get("id")
        .and_then(Value::as_u64)
        .and_then(|n| u32::try_from(n).ok())
    else {
        return Ok(());
    };
    let alive = world
        .resource::<SessionVehicles>()
        .get(id)
        .is_some_and(|r| r.is_alive());
    if alive {
        ops::switch_to(world, id).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ---- camera ----------------------------------------------------------------------------------

/// Data of the `camera` section.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CameraData {
    /// Mode as its number key (1 ego, 2 cockpit, 3 external, 4 passenger, 5 free).
    pub mode: u8,
    /// The mode before it (key 0).
    pub previous_mode: u8,
    /// Index into the sub-camera list of the mode.
    pub sub_camera: usize,
    /// Free camera speed (m/s).
    pub free_roam_speed: f32,
    /// Free camera yaw (rad).
    pub free_roam_yaw: f32,
    /// Free camera pitch (rad).
    pub free_roam_pitch: f32,
    /// Absolute OMSI position of the free camera `[x, y, z]` (m).
    pub free_roam_position: [f64; 3],
    /// External view distance (m).
    pub external_zoom: f32,
    /// External view yaw (rad).
    pub external_yaw: f32,
    /// External view pitch (rad).
    pub external_pitch: f32,
    /// External view orbit centre offset (m).
    pub pan_offset: [f32; 3],
    /// Look offsets in cockpit and passenger views (rad): yaw, pitch.
    pub view: [f32; 2],
    /// Look offsets in the ego view (rad): yaw, pitch.
    pub ego: [f32; 2],
}

fn mode_of(key: u8) -> Option<CameraMode> {
    CameraMode::ALL.into_iter().find(|m| m.key_number() == key)
}

fn camera_capture(world: &mut World) -> Result<Option<Value>, String> {
    let Some(state) = world.get_resource::<CameraState>().cloned() else {
        return Ok(None);
    };
    let translation = world
        .query_filtered::<&Transform, With<Camera3d>>()
        .iter(world)
        .next()
        .map_or(Vec3::ZERO, |t| t.translation);
    let omsi = opts_bevy::bevy_to_omsi_pos(translation);
    let (x, y) = ops::local_to_abs(world, Vec2::new(omsi.x, omsi.y));
    to_value(&CameraData {
        mode: state.mode.key_number(),
        previous_mode: state.previous_mode.key_number(),
        sub_camera: state.sub_camera_index,
        free_roam_speed: state.free_roam_speed,
        free_roam_yaw: state.free_roam_yaw,
        free_roam_pitch: state.free_roam_pitch,
        free_roam_position: [x, y, f64::from(omsi.z)],
        external_zoom: state.external_zoom,
        external_yaw: state.external_yaw,
        external_pitch: state.external_pitch,
        pan_offset: state.pan_offset.to_array(),
        view: [state.view_yaw, state.view_pitch],
        ego: [state.ego_yaw, state.ego_pitch],
    })
}

fn camera_apply(world: &mut World, data: &Value) -> Result<(), String> {
    let d: CameraData = parse(data, "camera")?;
    let wanted = mode_of(d.mode).unwrap_or(CameraMode::External);
    let allowed = world
        .get_resource::<CameraAvailability>()
        .is_none_or(|a| a.allows(wanted));
    let local = ops::abs_to_local(world, d.free_roam_position[0], d.free_roam_position[1]);
    let position = opts_bevy::omsi_to_bevy_pos(Vec3::new(
        local.x,
        local.y,
        d.free_roam_position[2] as f32,
    ));
    {
        let mut state = world.resource_mut::<CameraState>();
        // A mode the restored session cannot offer (the vehicle failed to come back) falls back
        // to the free camera at its saved place.
        state.mode = if allowed { wanted } else { CameraMode::FreeRoam };
        state.previous_mode = mode_of(d.previous_mode).unwrap_or(state.mode);
        state.sub_camera_index = d.sub_camera;
        state.free_roam_speed = d.free_roam_speed.max(opts_bevy::FREE_ROAM_MIN_SPEED);
        state.free_roam_yaw = d.free_roam_yaw;
        state.free_roam_pitch = d.free_roam_pitch;
        state.external_zoom = d.external_zoom;
        state.external_yaw = d.external_yaw;
        state.external_pitch = d.external_pitch;
        state.pan_offset = Vec3::from_array(d.pan_offset);
        state.view_yaw = d.view[0];
        state.view_pitch = d.view[1];
        state.ego_yaw = d.ego[0];
        state.ego_pitch = d.ego[1];
        if state.mode == CameraMode::FreeRoam {
            state.free_roam_spawn = position;
            state.free_roam_spawn_yaw = d.free_roam_yaw;
            state.free_roam_spawn_pitch = d.free_roam_pitch;
        }
    }
    if world.resource::<CameraState>().mode == CameraMode::FreeRoam {
        let rotation = Quat::from_euler(EulerRot::YXZ, d.free_roam_yaw, d.free_roam_pitch, 0.0);
        let mut cameras = world.query_filtered::<&mut Transform, With<Camera3d>>();
        for mut t in cameras.iter_mut(world) {
            t.translation = position;
            t.rotation = rotation;
        }
    }
    Ok(())
}

// ---- counters --------------------------------------------------------------------------------

/// Data of the `counters` section.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CountersData {
    /// Metres driven by the active vehicle since it was activated (the odometer).
    pub odometer_m: f64,
    /// Simulated seconds of the drive.
    pub duration_s: f32,
    /// Metres covered.
    pub distance_m: f32,
    /// Top speed (m/s).
    pub max_speed_m_s: f32,
}

fn counters_capture(world: &mut World) -> Result<Option<Value>, String> {
    let stats = world.get_resource::<SessionStats>().cloned().unwrap_or_default();
    to_value(&CountersData {
        odometer_m: world
            .get_resource::<RemoteControls>()
            .map_or(0.0, |r| r.odometer_m),
        duration_s: stats.duration_s,
        distance_m: stats.distance_m,
        max_speed_m_s: stats.max_speed_m_s,
    })
}

fn counters_apply(world: &mut World, data: &Value) -> Result<(), String> {
    let d: CountersData = parse(data, "counters")?;
    if let Some(mut stats) = world.get_resource_mut::<SessionStats>() {
        stats.duration_s = d.duration_s;
        stats.distance_m = d.distance_m;
        stats.max_speed_m_s = d.max_speed_m_s;
    }
    if let Some(mut remote) = world.get_resource_mut::<RemoteControls>() {
        remote.odometer_m = d.odometer_m;
        // Activating the vehicle restarts the odometer a frame later; it continues from here.
        remote.pending_odometer = Some((d.odometer_m, 5));
    }
    Ok(())
}

// ---- simulation ------------------------------------------------------------------------------

fn simulation_capture(world: &mut World) -> Result<Option<Value>, String> {
    let time = world.resource::<Time<Virtual>>();
    // The pause overlay pauses the clock itself; what the player had set is what counts.
    let by_overlay = world
        .get_resource::<Pause>()
        .is_some_and(|p| p.open && p.resume_time);
    Ok(Some(json!({
        "paused": time.is_paused() && !by_overlay,
        "time_scale": time.relative_speed(),
    })))
}

fn simulation_apply(world: &mut World, data: &Value) -> Result<(), String> {
    let mut time = world.resource_mut::<Time<Virtual>>();
    if let Some(scale) = data.get("time_scale").and_then(Value::as_f64) {
        time.set_relative_speed(scale as f32);
    }
    if data.get("paused").and_then(Value::as_bool) == Some(true) {
        time.pause();
    }
    Ok(())
}

/// Registers every section of this build.
pub fn register(app: &mut App) {
    let base = SaveSection {
        id: "",
        version: 1,
        stage: Stage::World,
        order: 0,
        required: false,
        capture: |_| Ok(None),
        prepare: None,
        apply: None,
        migrate: None,
        check: None,
        about: "",
    };
    app.register_save_section(SaveSection {
        id: "session",
        required: true,
        order: 0,
        capture: session_capture,
        prepare: Some(session_prepare),
        check: Some(session_check),
        about: "map folder and name, start point, mode, play time",
        ..base
    })
    .register_save_section(SaveSection {
        id: "time",
        order: 10,
        capture: time_capture,
        prepare: Some(time_prepare),
        apply: Some(time_apply),
        about: "date, clock base and simulated seconds",
        ..base
    })
    .register_save_section(SaveSection {
        id: "weather",
        order: 20,
        capture: weather_capture,
        prepare: Some(weather_prepare),
        about: "weather preset",
        ..base
    })
    .register_save_section(SaveSection {
        id: SAVE_SECTION,
        version: SAVE_SECTION_VERSION,
        order: 30,
        capture: vehicles_capture,
        apply: Some(vehicles_apply),
        migrate: Some(vehicles_migrate),
        check: Some(vehicles_check),
        about: "the session's vehicles with pose, velocity, inputs, gear and script variables",
        ..base
    })
    .register_save_section(SaveSection {
        id: "active_vehicle",
        stage: Stage::Settled,
        order: 0,
        capture: active_capture,
        apply: Some(active_apply),
        about: "which vehicle the driver is in",
        ..base
    })
    .register_save_section(SaveSection {
        id: "camera",
        stage: Stage::Settled,
        order: 10,
        capture: camera_capture,
        apply: Some(camera_apply),
        about: "camera mode, sub-camera, free camera pose, orbit and look offsets",
        ..base
    })
    .register_save_section(SaveSection {
        id: "counters",
        stage: Stage::Settled,
        order: 20,
        capture: counters_capture,
        apply: Some(counters_apply),
        about: "driven distance, session duration, top speed",
        ..base
    })
    .register_save_section(SaveSection {
        id: "simulation",
        stage: Stage::Settled,
        order: 100,
        capture: simulation_capture,
        apply: Some(simulation_apply),
        about: "paused state and time scale",
        ..base
    });
}

