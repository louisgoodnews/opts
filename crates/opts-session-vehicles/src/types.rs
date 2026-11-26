//! The value types of the session registry: ids, lifecycle, spawn requests and the record of one
//! session vehicle (with its live-state fields).

use opts_bevy::bevy::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use opts_world_stats::CameraSetup;

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
