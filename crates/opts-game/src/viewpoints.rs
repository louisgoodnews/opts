//! Stored viewpoints of the driver's view: one entry per vehicle and driver camera (M-0084).
//!
//! The file is `viewpoints.json` in the user data directory, next to `drive_history.json`:
//!
//! ```json
//! { "version": 1,
//!   "vehicles": { "man_sd77": { "cameras": { "0": { "x": 0.03, "y": -0.05, "z": 0.0, "fov_deg": 80.0 } } } } }
//! ```
//!
//! The vehicle key is the catalogue identity (the `.bus` file stem, lower case), so a repaint keeps
//! its viewpoint. Reading never fails the game: a missing file is an empty store, an unreadable
//! one is moved aside as `viewpoints.json.bad`, and an entry that is not valid (bad key, camera
//! index, non-finite or out-of-range value) is dropped with a warning while its neighbours stay.
//! Offsets are measured from the vehicle's own `[add_camera_driver]` preset, so an entry never
//! needs the vehicle's data and a vehicle without an entry shows its own viewpoint.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use bevy::prelude::*;
use opts_bevy::{
    CameraMode, CameraState, PlayerVehicle, Viewpoint, ViewpointRequest, ViewpointState,
};
use serde_json::{json, Value};

use crate::app_state::AppState;
use crate::hud::{keys as hud_keys, AlertSeverity, RaiseAlert};

/// The file format version this build reads and writes.
pub const FILE_VERSION: u64 = 1;

/// The keys of the player's vehicle (the components and the `.bus` path helpers) live in
/// `opts-game-state`; they are re-exported here under their old paths.
pub use opts_game_state::viewpoint_keys::{
    key_of_bus_file, model_of_bus_file, normalise_key, ViewpointKey, ViewpointModel,
};

/// Every stored viewpoint.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ViewpointStore {
    /// Viewpoints by vehicle key and driver camera index. An entry equal to the default is an
    /// explicit "the vehicle's own view" that overrides a model entry.
    pub vehicles: BTreeMap<String, BTreeMap<usize, Viewpoint>>,
    /// Viewpoints for every vehicle of a model (the vehicle's folder), by camera index.
    pub models: BTreeMap<String, BTreeMap<usize, Viewpoint>>,
}

type Section = BTreeMap<String, BTreeMap<usize, Viewpoint>>;

/// What reading a file found.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LoadReport {
    /// One line per problem (dropped entries, a file in a newer format).
    pub problems: Vec<String>,
    /// The whole file was not usable (not JSON or not an object) and should be moved aside.
    pub unreadable: bool,
}

impl ViewpointStore {
    /// Where the file lives: beside `drive_history.json`.
    #[must_use]
    pub fn path() -> Option<PathBuf> {
        crate::drive_history::DriveHistory::path().map(|p| p.with_file_name("viewpoints.json"))
    }

    /// The viewpoint of `key` and camera `index`, if one is stored.
    #[must_use]
    pub fn get(&self, key: &str, index: usize) -> Option<Viewpoint> {
        self.vehicles.get(key)?.get(&index).copied()
    }

    /// The viewpoint that applies to a vehicle: its own entry, else its model's.
    #[must_use]
    pub fn lookup(&self, key: &str, model: Option<&str>, index: usize) -> Option<Viewpoint> {
        self.get(key, index)
            .or_else(|| self.models.get(model?)?.get(&index).copied())
    }

    /// Stores a vehicle's viewpoint; the default removes the entry unless the vehicle's model has
    /// a viewpoint for this camera (then the default is kept as an explicit override).
    pub fn set_vehicle(&mut self, key: &str, model: Option<&str>, index: usize, vp: Viewpoint) {
        let vp = vp.clamped();
        let model_has =
            model.is_some_and(|m| self.models.get(m).is_some_and(|c| c.contains_key(&index)));
        if vp.is_default() && !model_has {
            Self::remove_from(&mut self.vehicles, key, index);
        } else {
            self.vehicles
                .entry(key.to_owned())
                .or_default()
                .insert(index, vp);
        }
    }

    /// Stores a viewpoint for a whole model and drops the vehicle's own override, so that the
    /// vehicle shows it too. Other vehicles' overrides stay.
    pub fn set_model(&mut self, key: &str, model: &str, index: usize, vp: Viewpoint) {
        let vp = vp.clamped();
        Self::remove_from(&mut self.vehicles, key, index);
        if vp.is_default() {
            Self::remove_from(&mut self.models, model, index);
        } else {
            self.models
                .entry(model.to_owned())
                .or_default()
                .insert(index, vp);
        }
    }

    /// Stores a viewpoint; the default (the vehicle's own view) removes the entry.
    pub fn set(&mut self, key: &str, index: usize, vp: Viewpoint) {
        self.set_vehicle(key, None, index, vp);
    }

    /// Removes a vehicle's entry (and the vehicle when it has no camera left).
    pub fn remove(&mut self, key: &str, index: usize) {
        Self::remove_from(&mut self.vehicles, key, index);
    }

    fn remove_from(section: &mut Section, key: &str, index: usize) {
        if let Some(cams) = section.get_mut(key) {
            cams.remove(&index);
            if cams.is_empty() {
                section.remove(key);
            }
        }
    }

    /// Reads one section (`vehicles` or `models`) of the file.
    fn parse_section(
        raw: Option<&Value>,
        name: &str,
        keep_default: bool,
        report: &mut LoadReport,
    ) -> Section {
        let mut out = Section::new();
        let Some(Value::Object(entries)) = raw else {
            return out;
        };
        for (raw_key, entry) in entries {
            let Some(key) = normalise_key(raw_key) else {
                report
                    .problems
                    .push(format!("{name} key {raw_key:?} is not valid"));
                continue;
            };
            let Some(cameras) = entry.get("cameras").and_then(Value::as_object) else {
                report.problems.push(format!("{key}: no cameras object"));
                continue;
            };
            for (raw_index, value) in cameras {
                let Ok(index) = raw_index.parse::<usize>() else {
                    report
                        .problems
                        .push(format!("{key}: camera index {raw_index:?} is not a number"));
                    continue;
                };
                match serde_json::from_value::<Viewpoint>(value.clone()) {
                    Ok(vp) if vp.is_valid() && (keep_default || !vp.is_default()) => {
                        out.entry(key.clone())
                            .or_default()
                            .insert(index, vp.clamped());
                    }
                    Ok(vp) if vp.is_valid() => {}
                    Ok(_) => report
                        .problems
                        .push(format!("{key} camera {index}: a value is out of range")),
                    Err(e) => report.problems.push(format!("{key} camera {index}: {e}")),
                }
            }
        }
        out
    }
}

mod io;

/// A viewpoint as JSON with the f32 noise rounded away (0.08, not 0.07999999821186066).
#[must_use]
pub fn vp_json(vp: &Viewpoint) -> Value {
    let r = |v: f32| (f64::from(v) * 10_000.0).round() / 10_000.0;
    json!({ "x": r(vp.x), "y": r(vp.y), "z": r(vp.z), "fov_deg": vp.fov_deg.map(r) })
}

mod plugin;
pub use plugin::{load_viewpoint, persist_viewpoint, sync_default_fov, ViewpointPlugin};

mod resource;
pub use resource::Viewpoints;

pub mod protocol;
pub mod share;

#[cfg(test)]
mod tests;
