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

/// Longest vehicle key kept.
const MAX_KEY_LEN: usize = 128;

/// The catalogue identity of the player's vehicle, on its root entity.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct ViewpointKey(pub String);

/// The key of a `.bus` file: its stem in lower case.
#[must_use]
pub fn key_of_bus_file(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_string_lossy();
    normalise_key(&stem)
}

/// Lower-cases and trims a vehicle key; `None` when it is empty, too long or has control characters.
#[must_use]
pub fn normalise_key(key: &str) -> Option<String> {
    let k = key.trim().to_lowercase();
    (!k.is_empty() && k.len() <= MAX_KEY_LEN && !k.chars().any(char::is_control)).then_some(k)
}

/// Every stored viewpoint.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ViewpointStore {
    /// Viewpoints by vehicle key and driver camera index.
    pub vehicles: BTreeMap<String, BTreeMap<usize, Viewpoint>>,
}

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

    /// Stores a viewpoint; the default (the vehicle's own view) removes the entry.
    pub fn set(&mut self, key: &str, index: usize, vp: Viewpoint) {
        let vp = vp.clamped();
        if vp.is_default() {
            self.remove(key, index);
        } else {
            self.vehicles
                .entry(key.to_owned())
                .or_default()
                .insert(index, vp);
        }
    }

    /// Removes an entry (and the vehicle when it has no camera left).
    pub fn remove(&mut self, key: &str, index: usize) {
        if let Some(cams) = self.vehicles.get_mut(key) {
            cams.remove(&index);
            if cams.is_empty() {
                self.vehicles.remove(key);
            }
        }
    }

    /// Parses file text; never fails. Invalid entries are dropped and listed in the report.
    #[must_use]
    pub fn parse(text: &str) -> (Self, LoadReport) {
        let mut store = Self::default();
        let mut report = LoadReport::default();
        let Ok(Value::Object(root)) = serde_json::from_str::<Value>(text) else {
            report.unreadable = true;
            report.problems.push("not a JSON object".to_owned());
            return (store, report);
        };
        match root.get("version").and_then(Value::as_u64) {
            Some(v) if v <= FILE_VERSION => {}
            other => {
                report.problems.push(format!(
                    "unsupported version {other:?}; its entries are ignored"
                ));
                return (store, report);
            }
        }
        let Some(Value::Object(vehicles)) = root.get("vehicles") else {
            return (store, report);
        };
        for (raw_key, entry) in vehicles {
            let Some(key) = normalise_key(raw_key) else {
                report
                    .problems
                    .push(format!("vehicle key {raw_key:?} is not valid"));
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
                    Ok(vp) if vp.is_valid() && !vp.is_default() => {
                        store
                            .vehicles
                            .entry(key.clone())
                            .or_default()
                            .insert(index, vp.clamped());
                    }
                    Ok(vp) if vp.is_default() => {}
                    Ok(_) => report
                        .problems
                        .push(format!("{key} camera {index}: a value is out of range")),
                    Err(e) => report.problems.push(format!("{key} camera {index}: {e}")),
                }
            }
        }
        (store, report)
    }

    /// The file text.
    #[must_use]
    pub fn to_json(&self) -> String {
        let vehicles: serde_json::Map<String, Value> = self
            .vehicles
            .iter()
            .map(|(key, cams)| {
                let cams: serde_json::Map<String, Value> = cams
                    .iter()
                    .map(|(i, vp)| (i.to_string(), vp_json(vp)))
                    .collect();
                (key.clone(), json!({ "cameras": cams }))
            })
            .collect();
        serde_json::to_string_pretty(&json!({ "version": FILE_VERSION, "vehicles": vehicles }))
            .unwrap_or_default()
    }

    /// Reads `path`. A missing file is an empty store; an unusable one is moved aside.
    #[must_use]
    pub fn load_from(path: &Path) -> Self {
        let Ok(text) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        let (store, report) = Self::parse(&text);
        for p in &report.problems {
            tracing::warn!(file = %path.display(), "viewpoints: {p}");
        }
        if report.unreadable {
            let bad = path.with_extension("json.bad");
            match std::fs::rename(path, &bad) {
                Ok(()) => {
                    tracing::warn!(moved_to = %bad.display(), "viewpoints: the file was unusable")
                }
                Err(e) => tracing::warn!("viewpoints: cannot move the unusable file aside: {e}"),
            }
        }
        store
    }

    /// Writes the store to `path` (through a temporary file, so a crash never leaves half a file).
    ///
    /// # Errors
    /// The I/O error when the file cannot be written.
    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, self.to_json())?;
        std::fs::rename(&tmp, path)
    }
}

/// A viewpoint as JSON with the f32 noise rounded away (0.08, not 0.07999999821186066).
#[must_use]
pub fn vp_json(vp: &Viewpoint) -> Value {
    let r = |v: f32| (f64::from(v) * 10_000.0).round() / 10_000.0;
    json!({ "x": r(vp.x), "y": r(vp.y), "z": r(vp.z), "fov_deg": vp.fov_deg.map(r) })
}

/// The store and where it lives (`None` when no user data directory is known).
#[derive(Resource, Debug, Clone, Default)]
pub struct Viewpoints {
    /// The file; tests point it at a temporary directory.
    pub path: Option<PathBuf>,
    /// What the file holds (as last read or written).
    pub store: ViewpointStore,
}

impl Viewpoints {
    /// Reads the default file.
    #[must_use]
    pub fn load() -> Self {
        let path = ViewpointStore::path();
        let store = path
            .as_deref()
            .map(ViewpointStore::load_from)
            .unwrap_or_default();
        Self { path, store }
    }

    /// Re-reads the file.
    pub fn reload(&mut self) {
        if let Some(p) = &self.path {
            self.store = ViewpointStore::load_from(p);
        }
    }

    /// Stores a viewpoint and writes the file.
    ///
    /// # Errors
    /// The I/O error when the file cannot be written; the in-memory store is updated anyway.
    pub fn save(&mut self, key: &str, index: usize, vp: Viewpoint) -> std::io::Result<()> {
        // Another run may have written other vehicles meanwhile: merge into the file's content.
        if let Some(p) = &self.path {
            let mut fresh = ViewpointStore::load_from(p);
            fresh.set(key, index, vp);
            self.store = fresh;
        } else {
            self.store.set(key, index, vp);
        }
        match &self.path {
            Some(p) => self.store.save_to(p),
            None => Ok(()),
        }
    }
}

/// Loads the stored viewpoint into the state when the vehicle or the camera changes and when the
/// driver's view is entered. An open editor is never touched.
pub fn load_viewpoint(
    camera: Res<CameraState>,
    vehicle: Query<&ViewpointKey, With<PlayerVehicle>>,
    mut viewpoints: ResMut<Viewpoints>,
    mut state: ResMut<ViewpointState>,
    mut was_cockpit: Local<bool>,
) {
    let key = vehicle.single().ok().map(|k| k.0.clone());
    let entered = camera.mode == CameraMode::Cockpit && !*was_cockpit;
    *was_cockpit = camera.mode == CameraMode::Cockpit;
    if state.vehicle_key != key {
        state.vehicle_key.clone_from(&key);
    }
    if state.editing {
        return;
    }
    let target = key.map(|k| (k, camera.sub_camera_index));
    if state.loaded_for == target && !entered {
        return;
    }
    if entered {
        viewpoints.reload();
    }
    let vp = target
        .as_ref()
        .and_then(|(k, i)| viewpoints.store.get(k, *i))
        .unwrap_or_default();
    if let Some((k, i)) = &target {
        tracing::debug!(vehicle = %k, camera = i, viewpoint = ?vp, "viewpoint loaded");
        state.camera_index = *i;
    }
    state.active = vp;
    state.committed = vp;
    state.loaded_for = target;
}

/// Writes what the editor saved (an exclusive system, so the protocol can run it on the spot).
pub fn persist_viewpoint(world: &mut World) {
    let Some(ViewpointRequest::Save(vp)) = world.resource::<ViewpointState>().request else {
        return;
    };
    let (key, index) = {
        let mut state = world.resource_mut::<ViewpointState>();
        state.request = None;
        (state.vehicle_key.clone(), state.camera_index)
    };
    let Some(key) = key else {
        return;
    };
    let result = world.resource_mut::<Viewpoints>().save(&key, index, vp);
    let (severity, text) = match result {
        Ok(()) => {
            tracing::info!(vehicle = %key, camera = index, viewpoint = ?vp, "viewpoint saved");
            (AlertSeverity::Info, hud_keys::ALERT_VIEWPOINT_SAVED)
        }
        Err(e) => {
            tracing::warn!(vehicle = %key, "cannot write the viewpoints file: {e}");
            (AlertSeverity::Warning, hud_keys::ALERT_VIEWPOINT_FAILED)
        }
    };
    world.write_message(RaiseAlert {
        id: "viewpoint.saved",
        severity,
        key: text,
        args: Vec::new(),
    });
}

/// Registers the store and its systems.
pub struct ViewpointPlugin;

impl Plugin for ViewpointPlugin {
    fn build(&self, app: &mut App) {
        if !app.world().contains_resource::<Viewpoints>() {
            app.insert_resource(Viewpoints::load());
        }
        app.add_systems(
            Update,
            (
                load_viewpoint.after(opts_bevy::camera::input::handle_camera_key_input),
                persist_viewpoint.after(opts_bevy::camera::viewpoint::handle_viewpoint_keys),
            )
                .run_if(in_state(AppState::InGame)),
        );
    }
}

pub mod protocol;

#[cfg(test)]
mod tests;
