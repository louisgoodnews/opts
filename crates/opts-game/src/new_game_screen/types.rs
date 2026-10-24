//! The data the wizard works on: catalogue lists and the choices of one drive.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Marker of the root node of the wizard screen.
#[derive(Component)]
pub struct NewGameScreenRoot;

/// A map of the OMSI installation as the lists show it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapItem {
    /// Folder name under `maps` (stable id).
    pub id: String,
    /// Folder name.
    pub name: String,
    /// Name the map calls itself.
    pub friendly_name: String,
    /// Description from the map, in the language its author wrote it in.
    pub description: String,
    /// Number of tiles (0 when unknown).
    pub tile_count: usize,
    /// Standard depot named by the map (empty when none).
    pub standard_depot: String,
    /// Years the map is set in, as written by the author (empty when none).
    pub years: String,
    /// Names of the start points.
    pub entry_points: Vec<String>,
}

/// A playable vehicle as the lists show it. Only facts the content provides are kept: a fact
/// without data is empty or `None`, never "N/A".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VehicleItem {
    /// `.bus` file stem (stable id).
    pub id: String,
    /// Manufacturer (may be empty).
    pub manufacturer: String,
    /// Model.
    pub model: String,
    /// Manufacturer and model as shown to the user.
    pub full_name: String,
    /// Default repaint.
    pub default_livery: String,
    /// `.bus` path relative to the OMSI root (empty when unknown).
    #[serde(default)]
    pub bus_file: String,
    /// Description from the vehicle (may be empty).
    pub description: String,
    /// `road`, `rail` or `aircraft`; empty when the content does not say.
    #[serde(default)]
    pub class: String,
    /// Empty weight in tonnes when the content gives it.
    #[serde(default)]
    pub mass_tonnes: Option<f64>,
}

impl VehicleItem {
    /// A vehicle with id, manufacturer and model (fixtures and tests).
    pub fn named(id: &str, manufacturer: &str, model: &str) -> Self {
        Self {
            id: id.to_string(),
            manufacturer: manufacturer.to_string(),
            model: model.to_string(),
            full_name: if manufacturer.is_empty() {
                model.to_string()
            } else {
                format!("{manufacturer} {model}")
            },
            default_livery: String::new(),
            bus_file: String::new(),
            description: String::new(),
            class: String::new(),
            mass_tonnes: None,
        }
    }
}

/// All maps and vehicles of the installation, as read from the catalogue.
#[derive(Resource, Debug, Clone, Default)]
pub struct GameCatalogue {
    /// Maps by friendly name.
    pub maps: Vec<MapItem>,
    /// Playable vehicles by manufacturer and model.
    pub vehicles: Vec<VehicleItem>,
}

impl GameCatalogue {
    /// The map with this friendly name.
    pub fn map_by_name(&self, name: &str) -> Option<&MapItem> {
        self.maps.iter().find(|m| m.friendly_name == name)
    }

    /// The vehicle a `NewGameSetup::selected_vehicle` key names: its display name, or its id when
    /// several vehicles share a display name.
    pub fn find_vehicle(&self, key: &str) -> Option<&VehicleItem> {
        self.vehicles
            .iter()
            .find(|v| v.id == key)
            .or_else(|| self.vehicles.iter().find(|v| v.full_name == key))
    }

    /// The key that identifies `vehicle` in `NewGameSetup::selected_vehicle`: the display name
    /// when it is unique, the id otherwise.
    pub fn vehicle_key(&self, vehicle: &VehicleItem) -> String {
        let shared = self
            .vehicles
            .iter()
            .filter(|v| v.full_name == vehicle.full_name)
            .count()
            > 1;
        if shared {
            vehicle.id.clone()
        } else {
            vehicle.full_name.clone()
        }
    }
}

/// The choices of the drive that is being planned.
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct NewGameSetup {
    /// Friendly name of the map.
    pub selected_map: String,
    /// Start point name.
    pub selected_entry_point: String,
    /// Vehicle key (see [`GameCatalogue::vehicle_key`]); empty until one is chosen.
    pub selected_vehicle: String,
    /// *Drive without a vehicle*: the session starts with the map only (free camera, no
    /// instruments); `selected_vehicle` is then ignored.
    pub no_vehicle: bool,
    /// Chosen repaint; empty for the default.
    pub selected_livery: String,
    /// Weather preset id.
    pub weather_preset: String,
    /// Year.
    pub year: i32,
    /// Month 1-12.
    pub month: u32,
    /// Day 1-31.
    pub day: u32,
    /// Hour 0-23.
    pub hour: u32,
    /// Minute 0-59.
    pub minute: u32,
    /// Equipment options set on the vehicle's scripts.
    pub custom_setvars: HashMap<String, i32>,
    /// A composed train: every car's `.bus` path (relative to the OMSI root) and whether it is
    /// turned, the first being the chosen vehicle. Empty for a drive with the vehicle alone.
    pub formation: Vec<(String, bool)>,
}

impl Default for NewGameSetup {
    fn default() -> Self {
        let today = super::planner::today();
        Self {
            selected_map: "Berlin-Spandau".to_string(),
            // No map-specific default: the wizard picks the first start point of the chosen map
            // and the preferred vehicle once the catalogue is there (`planner::ensure_valid`).
            selected_entry_point: String::new(),
            selected_vehicle: String::new(),
            no_vehicle: false,
            selected_livery: String::new(),
            weather_preset: "Clear Summer Day".to_string(),
            // Today; choosing a map sets the map's default date (`planner::default_map_date`).
            year: today.year,
            month: today.month,
            day: today.day,
            hour: 8,
            minute: 30,
            custom_setvars: HashMap::new(),
            formation: Vec::new(),
        }
    }
}
