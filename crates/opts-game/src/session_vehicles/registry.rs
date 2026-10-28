//! The registry of the vehicles the user spawned in a session ([`SessionVehicles`]).
//!
//! A session is a map plus zero or more *user* vehicles. Every vehicle the user brings in (the
//! one chosen in the wizard, the ones spawned later from the pause menu or through the control
//! protocol) gets a stable session id and a [`SessionVehicleRecord`]; parked and AI vehicles are
//! not part of it. The registry is plain data (it serialises as one section of the later session
//! save, [`SAVE_SECTION`]); the live entities are runtime-only fields that the systems of this
//! module keep in step. The lifecycle and the switch and despawn rules are in
//! `docs/src/architecture/session-vehicles.md`.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::in_game::omsi_world::CameraSetup;

/// Stable id of a session vehicle (never reused inside a session).
pub type VehicleId = u32;

/// Name of the registry's section in the session save (M-0069).
pub const SAVE_SECTION: &str = "session_vehicles";
/// Format version of the section.
pub const SAVE_SECTION_VERSION: u32 = 2;

/// Component on the root entity of every session vehicle. It is the link from an entity back to its
/// registry record; the *active* vehicle additionally carries [`opts_bevy::PlayerVehicle`].
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionVehicle(pub VehicleId);

/// Where in its life a session vehicle is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lifecycle {
    /// Asked for; waiting for the spawner (or its meshes are still decoding).
    Spawning,
    /// In the world.
    Alive,
    /// Gone (see [`DespawnReason`]); the record stays for the session's log.
    Despawned,
    /// The spawn failed (see the record's `error`).
    Failed,
}

/// Why a vehicle is gone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DespawnReason {
    /// The user (or the control protocol) despawned it.
    User,
    /// Its entity vanished without a request (destroyed, or removed by streaming); a warning toast
    /// told the user.
    Lost,
    /// A spawn request was cancelled before the vehicle existed.
    Cancelled,
}

/// Which entry point of the map a spawn asks for.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntrySel {
    /// The entry point the session was started at.
    #[default]
    Default,
    /// The `[entrypoints]` record with this number.
    Id(u32),
    /// The record with this name (exact, case-insensitive, else the only one containing it).
    Name(String),
    /// The bus stop object with this station id (`Busstops.cfg`), the vehicle's front at the
    /// stop; refused while another vehicle stands there (M-0145).
    Stop(u32),
}

/// A place in the world: absolute OMSI coordinates (metres) and compass heading (degrees).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SpawnPoint {
    /// East (m).
    pub x: f64,
    /// North (m).
    pub y: f64,
    /// Height (m).
    pub z: f64,
    /// Heading, clockwise from north (degrees).
    pub heading_deg: f64,
}

/// Everything a spawn request carries.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SpawnParams {
    /// Catalogue id of the vehicle (its `.bus` file stem); empty when unknown.
    pub catalogue_id: String,
    /// Display name.
    pub name: String,
    /// The `.bus` file.
    pub bus_file: String,
    /// Repaint (empty: the vehicle's default).
    pub repaint: String,
    /// Equipment options set on the vehicle's scripts.
    pub setvars: Vec<(String, i32)>,
    /// The entry point to spawn at.
    pub entry: EntrySel,
    /// A train formation: `bus_file` is then its first car and the other cars are coupled behind it.
    pub formation: Option<opts_core::formats::trains::TrainFormation>,
}

/// One session vehicle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionVehicleRecord {
    /// Stable session id.
    pub id: VehicleId,
    /// Catalogue id (`.bus` file stem); empty when the vehicle came from a bare path.
    pub catalogue_id: String,
    /// Display name.
    pub name: String,
    /// The `.bus` file it was built from.
    pub bus_file: String,
    /// Repaint (empty: the default of the vehicle).
    pub repaint: String,
    /// Equipment options set on its scripts.
    pub setvars: Vec<(String, i32)>,
    /// The entry point that was asked for.
    pub entry: EntrySel,
    /// Record number of the entry point it was spawned at (`None`: no entry resolved).
    pub spawn_entry_id: Option<u32>,
    /// Name of that entry point as `global.cfg` gives it (empty: none). The HUD place label shows it.
    pub spawn_entry_name: String,
    /// Where it was put.
    pub spawn_point: Option<SpawnPoint>,
    /// Where in its life it is.
    pub lifecycle: Lifecycle,
    /// Why it is gone.
    pub despawn_reason: Option<DespawnReason>,
    /// What went wrong when `lifecycle` is [`Lifecycle::Failed`].
    pub error: Option<String>,
    /// The vehicle's live state: written by the session save, and, after a restore, waiting to be
    /// put onto the rebuilt vehicle (cleared once applied).
    #[serde(default)]
    pub saved_state: Option<super::state::VehicleState>,
    /// Path of the `.bus` file relative to the OMSI installation, when it lies below it (a save
    /// moved to another computer finds the vehicle again through it).
    #[serde(default)]
    pub bus_rel: Option<String>,
    /// The train formation the vehicle was built from (`None` for a single vehicle); a save
    /// restores the whole train from it.
    #[serde(default)]
    pub formation: Option<opts_core::formats::trains::TrainFormation>,
    /// The root entity while alive.
    #[serde(skip)]
    pub entity: Option<Entity>,
    /// The following units (trailer chain, tram cars) while alive.
    #[serde(skip)]
    pub trailers: Vec<Entity>,
    /// Camera parameters derived from the vehicle (chase distance, cockpit offset).
    #[serde(skip)]
    pub camera: Option<CameraSetup>,
    /// The vehicle has been further than the place-label distance from its spawn point since it
    /// was spawned or last respawned: the label stays hidden.
    #[serde(skip)]
    pub place_dismissed: bool,
    /// Frames the entity has been missing since the record became alive (a vehicle that was just
    /// built exists only once the spawner's commands are applied).
    #[serde(skip)]
    pub missing_frames: u32,
}

impl SessionVehicleRecord {
    /// Whether the vehicle is in the world.
    #[must_use]
    pub fn is_alive(&self) -> bool {
        self.lifecycle == Lifecycle::Alive
    }

    /// The record as the control protocol and the save write it.
    #[must_use]
    pub fn describe(&self, active: bool) -> Value {
        json!({
            "id": self.id,
            "catalogue_id": self.catalogue_id,
            "name": self.name,
            "bus_file": self.bus_file,
            "repaint": self.repaint,
            "setvars": self.setvars.iter().map(|(k, v)| json!({ "name": k, "value": v })).collect::<Vec<_>>(),
            "entry": self.entry,
            "spawn_entry_id": self.spawn_entry_id,
            "spawn_entry_name": self.spawn_entry_name,
            "spawn_point": self.spawn_point,
            "lifecycle": self.lifecycle,
            "despawn_reason": self.despawn_reason,
            "error": self.error,
            "active": active,
            "trailers": self.trailers.len(),
        })
    }
}

/// The user's vehicles in this session, in the order they were requested.
#[derive(Resource, Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionVehicles {
    next_id: VehicleId,
    active: Option<VehicleId>,
    records: Vec<SessionVehicleRecord>,
}

impl SessionVehicles {
    /// Forgets everything (a session starts or ends).
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// All records, oldest first.
    #[must_use]
    pub fn records(&self) -> &[SessionVehicleRecord] {
        &self.records
    }

    /// The record with this id.
    #[must_use]
    pub fn get(&self, id: VehicleId) -> Option<&SessionVehicleRecord> {
        self.records.iter().find(|r| r.id == id)
    }

    /// The record with this id, mutable.
    pub fn get_mut(&mut self, id: VehicleId) -> Option<&mut SessionVehicleRecord> {
        self.records.iter_mut().find(|r| r.id == id)
    }

    /// The id of the active vehicle (the one that takes the driver's input).
    #[must_use]
    pub fn active_id(&self) -> Option<VehicleId> {
        self.active
    }

    /// The active vehicle's record.
    #[must_use]
    pub fn active(&self) -> Option<&SessionVehicleRecord> {
        self.active.and_then(|id| self.get(id))
    }

    /// Records a live change of an equipment option of a vehicle: kept for the save, a saved
    /// configuration and the status of the editor. A value equal to the repaint's own `default`
    /// removes the override; the list stays sorted by name. `false` when there is no such vehicle.
    pub fn set_setvar(&mut self, id: VehicleId, name: &str, value: i32, default: i32) -> bool {
        let Some(r) = self.get_mut(id) else {
            return false;
        };
        r.setvars.retain(|(n, _)| !n.eq_ignore_ascii_case(name));
        if value != default {
            r.setvars.push((name.to_owned(), value));
            r.setvars.sort();
        }
        true
    }

    /// Makes `id` the active vehicle (or none).
    pub fn set_active(&mut self, id: Option<VehicleId>) {
        self.active = id;
    }

    /// Ids of the vehicles in the world, oldest first.
    #[must_use]
    pub fn alive_ids(&self) -> Vec<VehicleId> {
        self.records
            .iter()
            .filter(|r| r.is_alive())
            .map(|r| r.id)
            .collect()
    }

    /// Number of vehicles that are in the world or on their way.
    #[must_use]
    pub fn live_count(&self) -> usize {
        self.records
            .iter()
            .filter(|r| matches!(r.lifecycle, Lifecycle::Alive | Lifecycle::Spawning))
            .count()
    }

    fn new_record(&mut self, params: SpawnParams, lifecycle: Lifecycle) -> VehicleId {
        self.next_id += 1;
        let id = self.next_id;
        self.records.push(SessionVehicleRecord {
            id,
            catalogue_id: params.catalogue_id,
            name: params.name,
            bus_file: params.bus_file,
            repaint: params.repaint,
            setvars: params.setvars,
            entry: params.entry,
            spawn_entry_id: None,
            spawn_entry_name: String::new(),
            spawn_point: None,
            lifecycle,
            despawn_reason: None,
            error: None,
            saved_state: None,
            bus_rel: None,
            formation: params.formation,
            entity: None,
            trailers: Vec::new(),
            camera: None,
            place_dismissed: false,
            missing_frames: 0,
        });
        id
    }

    /// Queues a vehicle for the spawner and returns its id.
    pub fn request_spawn(&mut self, params: SpawnParams) -> VehicleId {
        self.new_record(params, Lifecycle::Spawning)
    }

    /// The oldest request nobody works on yet.
    #[must_use]
    pub fn next_pending(&self, in_flight: Option<VehicleId>) -> Option<VehicleId> {
        self.records
            .iter()
            .find(|r| r.lifecycle == Lifecycle::Spawning && Some(r.id) != in_flight)
            .map(|r| r.id)
    }

    /// Records where a vehicle was put and that it exists now.
    pub fn mark_alive(
        &mut self,
        id: VehicleId,
        entity: Entity,
        entry: Option<(u32, String)>,
        point: Option<SpawnPoint>,
        camera: Option<CameraSetup>,
    ) {
        if let Some(r) = self.get_mut(id) {
            r.lifecycle = Lifecycle::Alive;
            r.entity = Some(entity);
            r.spawn_entry_id = entry.as_ref().map(|(i, _)| *i);
            r.spawn_entry_name = entry.map(|(_, n)| n).unwrap_or_default();
            r.spawn_point = point;
            r.camera = camera;
            r.place_dismissed = false;
            r.missing_frames = 0;
            r.error = None;
        }
    }

    /// Records that a spawn failed.
    pub fn mark_failed(&mut self, id: VehicleId, error: impl Into<String>) {
        if let Some(r) = self.get_mut(id) {
            r.lifecycle = Lifecycle::Failed;
            r.error = Some(error.into());
            r.entity = None;
        }
        if self.active == Some(id) {
            self.active = None;
        }
    }

    /// Records that a vehicle is gone; it stops being the active one.
    pub fn mark_despawned(&mut self, id: VehicleId, reason: DespawnReason) {
        if let Some(r) = self.get_mut(id) {
            r.lifecycle = Lifecycle::Despawned;
            r.despawn_reason = Some(reason);
            r.entity = None;
            r.trailers.clear();
        }
        if self.active == Some(id) {
            self.active = None;
        }
    }

    /// The vehicle that becomes active when `gone` (the active one) is removed: the most recently
    /// spawned other vehicle that is in the world, or none.
    #[must_use]
    pub fn successor_of(&self, gone: VehicleId) -> Option<VehicleId> {
        self.records
            .iter()
            .rev()
            .find(|r| r.is_alive() && r.id != gone)
            .map(|r| r.id)
    }

    /// The vehicle `step` places after the active one among the living ones (wrapping); with no
    /// active vehicle the first (or last, for a negative step).
    #[must_use]
    pub fn cycle(&self, step: i32) -> Option<VehicleId> {
        let alive = self.alive_ids();
        if alive.is_empty() {
            return None;
        }
        let len = alive.len() as i32;
        let at = self
            .active
            .and_then(|a| alive.iter().position(|id| *id == a))
            .map(|p| p as i32);
        let next = match at {
            Some(p) => (p + step).rem_euclid(len),
            None if step < 0 => len - 1,
            None => 0,
        };
        alive.get(next as usize).copied()
    }

    /// Registers a vehicle that was built outside the spawn queue (the session's first vehicle).
    pub fn register_alive(
        &mut self,
        params: SpawnParams,
        entity: Entity,
        entry: Option<(u32, String)>,
        point: Option<SpawnPoint>,
        camera: Option<CameraSetup>,
    ) -> VehicleId {
        let id = self.new_record(params, Lifecycle::Alive);
        self.mark_alive(id, entity, entry, point, camera);
        id
    }

    /// The registry as the control protocol answers it.
    #[must_use]
    pub fn describe(&self) -> Value {
        json!({
            "active": self.active,
            "count": self.live_count(),
            "vehicles": self
                .records
                .iter()
                .map(|r| r.describe(self.active == Some(r.id)))
                .collect::<Vec<_>>(),
        })
    }

    /// The registry as a section of the session save: the vehicles that are in the world or on
    /// their way (the despawned ones are history, not state). The runtime fields are left out.
    #[must_use]
    pub fn to_section(&self) -> Value {
        let live: Vec<&SessionVehicleRecord> = self
            .records
            .iter()
            .filter(|r| matches!(r.lifecycle, Lifecycle::Alive | Lifecycle::Spawning))
            .collect();
        json!({
            "version": SAVE_SECTION_VERSION,
            "next_id": self.next_id,
            "active": self.active.filter(|a| live.iter().any(|r| r.id == *a)),
            "vehicles": live,
        })
    }

    /// Reads a section written by [`Self::to_section`]. The vehicles come back as spawn requests
    /// ([`Lifecycle::Spawning`]) at their saved spawn point; the spawner builds them again and the
    /// saved active id is restored once it exists.
    ///
    /// # Errors
    /// The section has another version or is not shaped like one.
    pub fn from_section(section: &Value) -> Result<Self, String> {
        let version = section.get("version").and_then(Value::as_u64);
        if version != Some(u64::from(SAVE_SECTION_VERSION)) {
            return Err(format!("unsupported session_vehicles version {version:?}"));
        }
        let records: Vec<SessionVehicleRecord> = serde_json::from_value(
            section
                .get("vehicles")
                .cloned()
                .unwrap_or(Value::Array(vec![])),
        )
        .map_err(|e| e.to_string())?;
        let next_id = section
            .get("next_id")
            .and_then(Value::as_u64)
            .and_then(|n| u32::try_from(n).ok())
            .unwrap_or(0)
            .max(records.iter().map(|r| r.id).max().unwrap_or(0));
        let active = section
            .get("active")
            .and_then(Value::as_u64)
            .and_then(|n| u32::try_from(n).ok());
        Ok(Self {
            next_id,
            active,
            records: records
                .into_iter()
                .map(|mut r| {
                    r.lifecycle = Lifecycle::Spawning;
                    r
                })
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests;
