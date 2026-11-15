//! The map change's types: the vehicle choice, the errors, the plan and its inputs.

use super::*;

/// What to do with the vehicle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VehicleChoice {
    /// Build the active vehicle again on the other map.
    #[default]
    Carry,
    /// Go on without a vehicle (free camera).
    None,
}

/// Why a map change cannot be planned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeMapError {
    /// No drive is running (or it is still loading).
    NotInDrive,
    /// The catalogue is not there yet.
    NoCatalogue,
    /// No map matches the text.
    UnknownMap(String),
    /// The text matches several maps.
    AmbiguousMap(String, Vec<String>),
    /// The target map has no such entry point.
    UnknownEntry {
        /// The map.
        map: String,
        /// What was asked for.
        entry: String,
        /// What the map offers.
        available: Vec<String>,
    },
    /// The active vehicle is not in the catalogue (it cannot be built on the other map).
    VehicleUnavailable(String),
}

impl std::fmt::Display for ChangeMapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotInDrive => write!(f, "no drive is running"),
            Self::NoCatalogue => write!(f, "the catalogue is not ready yet"),
            Self::UnknownMap(m) => write!(f, "no installed map matches `{m}`"),
            Self::AmbiguousMap(m, all) => {
                write!(f, "`{m}` matches several maps: {}", all.join(", "))
            }
            Self::UnknownEntry {
                map,
                entry,
                available,
            } => write!(
                f,
                "map `{map}` has no entry point `{entry}` (it has: {})",
                available.join(", ")
            ),
            Self::VehicleUnavailable(v) => write!(
                f,
                "the vehicle `{v}` is not in the catalogue and cannot be built on the other map"
            ),
        }
    }
}

/// What a map change does, decided before anything is touched.
#[derive(Debug, Clone, PartialEq)]
pub struct ChangeMapPlan {
    /// Folder name of the target map.
    pub map_id: String,
    /// Friendly name of the target map (what the setup stores).
    pub map: String,
    /// The start point.
    pub entry_point: String,
    /// The start point was not asked for: the map's first one is used.
    pub entry_defaulted: bool,
    /// The vehicle key of the setup (empty without a vehicle).
    pub vehicle_key: String,
    /// Display name of the vehicle (empty without one).
    pub vehicle_name: String,
    /// The session goes on without a vehicle.
    pub no_vehicle: bool,
    /// The repaint of the vehicle.
    pub repaint: String,
    /// The equipment options of the vehicle.
    pub setvars: Vec<(String, i32)>,
    /// The date of the session clock.
    pub date: CalendarDate,
    /// The time of day of the session clock, seconds.
    pub seconds_of_day: f64,
    /// The weather preset.
    pub weather: String,
    /// Session vehicles that are not carried over (their names).
    pub dropped: Vec<String>,
}

impl ChangeMapPlan {
    /// Writes the plan into the setup the loading pipeline reads.
    pub fn apply_to(&self, setup: &mut NewGameSetup) {
        setup.selected_map.clone_from(&self.map);
        setup.selected_entry_point.clone_from(&self.entry_point);
        setup.no_vehicle = self.no_vehicle;
        if !self.no_vehicle {
            setup.selected_vehicle.clone_from(&self.vehicle_key);
            setup.selected_livery.clone_from(&self.repaint);
            setup.custom_setvars = self.setvars.iter().cloned().collect();
        }
        setup.weather_preset.clone_from(&self.weather);
        setup.year = self.date.year;
        setup.month = self.date.month;
        setup.day = self.date.day;
        let minutes = (self.seconds_of_day / 60.0).floor().max(0.0) as u32;
        setup.hour = (minutes / 60) % 24;
        setup.minute = minutes % 60;
    }

    /// The plan as JSON (the protocol's answer).
    #[must_use]
    pub fn describe(&self) -> serde_json::Value {
        serde_json::json!({
            "map": self.map,
            "map_id": self.map_id,
            "entry_point": self.entry_point,
            "entry_defaulted": self.entry_defaulted,
            "vehicle": if self.no_vehicle { serde_json::Value::Null } else { self.vehicle_name.clone().into() },
            "repaint": self.repaint,
            "date": format!("{:04}-{:02}-{:02}", self.date.year, self.date.month, self.date.day),
            "time": format!("{:02}:{:02}", (self.seconds_of_day / 3600.0) as u32 % 24, (self.seconds_of_day / 60.0) as u32 % 60),
            "weather": self.weather,
            "dropped": self.dropped,
        })
    }
}

/// What the plan reads from the running session.
#[derive(Debug, Clone)]
pub struct PlanInputs<'a> {
    /// The installed maps and vehicles.
    pub catalogue: &'a GameCatalogue,
    /// The vehicle that is active (`None`: the session has none).
    pub active: Option<&'a SessionVehicleRecord>,
    /// Names of the other living session vehicles.
    pub others: Vec<String>,
    /// The session clock: date, time of day in seconds, weather preset.
    pub clock: (CalendarDate, f64, String),
}
