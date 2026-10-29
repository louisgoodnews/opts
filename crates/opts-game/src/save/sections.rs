//! The sections this build saves. Each one is registered like any feature's would be; the save
//! system (`container`, `store`, `registry`, `ops`, `restore`) does not know them.
//!
//! | id | holds | restored |
//! | :-- | :-- | :-- |
//! | `session` | map folder and name, start point, mode, play time | before the session starts |
//! | `time` | start date and clock, play time, the environment's date, time, map date, weather and rate | map date and start before; the clock when everything is back |
//! | `weather` | the session's weather preset | before |
//! | `session_vehicles` | the registry with every vehicle's live state | vehicles rebuilt by the spawner |
//! | `active_vehicle` | which vehicle the driver is in | when all vehicles exist |
//! | `camera` | mode, sub-camera, free camera pose, orbit and look offsets | when all vehicles exist |
//! | `counters` | driven distance, session duration and top speed | when all vehicles exist |
//! | `passenger_load` | the test count of people the vehicle's dynamics carry, if one is set | when all vehicles exist |
//! | `simulation` | paused, time scale | last |

use std::path::Path;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::registry::{CheckEnv, Issue, IssueKind, SaveAppExt, SaveSection, Stage};
use super::sections_runtime as runtime;
use super::{fingerprint, PlayClock};
use crate::in_game::omsi_world::WorldLoadJob;
use crate::in_game::types::InGameSessionInfo;
use crate::new_game_screen::types::NewGameSetup;
use crate::session_vehicles::{ops, EntrySel, SessionVehicles, SAVE_SECTION, SAVE_SECTION_VERSION};
use opts_bevy::SessionEnvironment;

pub(super) fn parse<T: for<'de> Deserialize<'de>>(data: &Value, what: &str) -> Result<T, String> {
    serde_json::from_value(data.clone()).map_err(|e| format!("{what}: {e}"))
}

pub(super) fn to_value<T: Serialize>(value: &T) -> Result<Option<Value>, String> {
    serde_json::to_value(value)
        .map(Some)
        .map_err(|e| e.to_string())
}

/// Where a saved vehicle's `.bus` file is now: the saved path if it exists, else the path relative
/// to the installation `root`.
#[must_use]
pub fn resolve_bus_file(
    saved: &str,
    rel: Option<&str>,
    root: Option<&Path>,
) -> Option<std::path::PathBuf> {
    let direct = std::path::PathBuf::from(saved);
    if direct.is_file() {
        return Some(direct);
    }
    let root = root?;
    // A relative saved path, or the recorded relative one, below the installation.
    [
        Some(Path::new(saved)).filter(|p| p.is_relative()),
        rel.map(Path::new),
    ]
    .into_iter()
    .flatten()
    .map(|p| root.join(p))
    .find(|p| p.is_file())
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
    let map_folder = match world.get_resource::<WorldLoadJob>() {
        Some(job) => job
            .map_dir()
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .ok_or("the map folder has no name")?,
        // No loader (a headless test world): the setup's map is the folder.
        None => world.resource::<NewGameSetup>().selected_map.clone(),
    };
    let info = world.resource::<InGameSessionInfo>();
    to_value(&SessionData {
        map_folder,
        map_name: info.map_name.clone(),
        entry_point: info.entry_point.clone(),
        mode: "drive".to_owned(),
        played_s: world
            .get_resource::<PlayClock>()
            .map_or(0.0, |c| c.played_s),
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

// ---- time (`sections_time`) -----------------------------------------------------------------

pub use super::sections_time::TimeData;
use super::sections_time::{time_apply, time_capture, time_prepare};

// ---- weather ---------------------------------------------------------------------------------

fn weather_capture(world: &mut World) -> Result<Option<Value>, String> {
    // The weather of the session now (the pause page may have changed it), else the plan's.
    let preset = world.get_resource::<SessionEnvironment>().map_or_else(
        || world.resource::<NewGameSetup>().weather_preset.clone(),
        |e| e.weather().to_owned(),
    );
    Ok(Some(json!({ "preset": preset })))
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
        .map(|j| j.omsi_root().to_path_buf())
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
                if let (Some(old), Some(now)) = (
                    env.fingerprint.vehicles.get(&key),
                    fingerprint::vehicle(&path),
                ) {
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
        // Last of the clock's users: the clock ran while the vehicles were rebuilt.
        stage: Stage::Settled,
        order: 5,
        capture: time_capture,
        prepare: Some(time_prepare),
        apply: Some(time_apply),
        about:
            "start date and clock, play time, the clock's date, time, map date, weather and rate",
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
        stage: Stage::Vehicles,
        order: 0,
        capture: active_capture,
        apply: Some(active_apply),
        about: "which vehicle the driver is in",
        ..base
    });
    runtime::register(app, base);
}
