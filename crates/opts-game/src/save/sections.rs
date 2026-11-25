//! The sections this build saves. Each one is registered like any feature's would be; the save
//! system (`container`, `store`, `registry`, `ops`, `restore`) does not know them.
//!
//! | id | holds | restored |
//! | :-- | :-- | :-- |
//! | `session` | map folder and name, start point, mode, play time | before the session starts |
//! | `time` | start date and clock, play time, the environment's date, time, map date, weather and rate | map date and start before; the clock when everything is back |
//! | `weather` | the session's weather preset | before |
//! | `session_vehicles` | the registry with every vehicle's live state | vehicles rebuilt by the spawner |
//! | `fuel` | the fuel or battery charge of each session vehicle (M-0123) | when all vehicles exist |
//! | `active_vehicle` | which vehicle the driver is in | when all vehicles exist |
//! | `camera` | mode, sub-camera, free camera pose, orbit and look offsets | when all vehicles exist |
//! | `counters` | driven distance, session duration and top speed | when all vehicles exist |
//! | `passenger_load` | the test count of people the vehicle's dynamics carry, if one is set | when all vehicles exist |
//! | `career` | money, owned vehicles, finished shifts, experience and the running shift (ISS-047) | with the counters |
//! | `simulation` | paused, time scale | last |

use std::path::Path;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::registry::{SaveAppExt, SaveSection, Stage};
use super::sections_runtime as runtime;
use crate::session_vehicles::{SAVE_SECTION, SAVE_SECTION_VERSION};

mod career;
mod fuel;
mod paths;
mod session;
mod vehicles;

use fuel::{fuel_apply, fuel_capture};
use session::{session_capture, session_check, session_prepare, weather_capture, weather_prepare};
use vehicles::{
    active_apply, active_capture, vehicles_apply, vehicles_capture, vehicles_check,
    vehicles_migrate,
};

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
    if paths::bus_path_allowed(&direct, root) && direct.is_file() {
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

// ---- time (`sections_time`) -----------------------------------------------------------------

pub use super::sections_time::TimeData;
use super::sections_time::{time_apply, time_capture, time_prepare};

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
        id: "fuel",
        version: 1,
        stage: Stage::Vehicles,
        order: 10,
        capture: fuel_capture,
        apply: Some(fuel_apply),
        about: "fuel or battery charge of each session vehicle",
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
    })
    .register_save_section(career::section(base));
    runtime::register(app, base);
}
