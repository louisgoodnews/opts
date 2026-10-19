//! Changing the map inside a running drive (M-0108).
//!
//! The Alt menu's *Change map* page and the control protocol's `session.change_map` both end
//! here: the rules of what is carried over to the other map are a pure function ([`plan`]), and
//! [`start`] writes the plan into the session setup and enters the session again through the
//! same teardown and loading path as *Load game* (`save::restore::begin_restore`), so no second
//! loading path exists.
//!
//! Carry-over: the active vehicle (same catalogue id, repaint and equipment) is built again at an
//! entry point of the target map; the session's date, time of day and weather are kept; the
//! other session vehicles are dropped and listed in the plan; a session without a vehicle stays
//! without one. Where the vehicle cannot be carried (it is not in the catalogue any more) the
//! plan fails with [`ChangeMapError::VehicleUnavailable`], and the caller asks the player: choose
//! another vehicle from the menu first, or [`VehicleChoice::None`] to go on without one.

use bevy::prelude::*;
use opts_bevy::environment::{CalendarDate, SessionEnvironment};

use crate::app_state::AppState;
use crate::new_game_screen::types::{GameCatalogue, MapItem, NewGameSetup, VehicleItem};
use crate::session_vehicles::{SessionVehicleRecord, SessionVehicles};

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

/// The maps whose id, folder or friendly name equals `text` (case-insensitively), else those that
/// contain it.
#[must_use]
pub fn find_maps<'c>(catalogue: &'c GameCatalogue, text: &str) -> Vec<&'c MapItem> {
    let t = text.trim().to_lowercase();
    let exact: Vec<&MapItem> = catalogue
        .maps
        .iter()
        .filter(|m| {
            [&m.id, &m.name, &m.friendly_name]
                .iter()
                .any(|n| n.to_lowercase() == t)
        })
        .collect();
    if !exact.is_empty() {
        return exact;
    }
    catalogue
        .maps
        .iter()
        .filter(|m| {
            [&m.id, &m.name, &m.friendly_name]
                .iter()
                .any(|n| n.to_lowercase().contains(&t))
        })
        .collect()
}

/// The entry point of `map` that `text` names (exact, else the only one that contains it).
fn find_entry<'m>(map: &'m MapItem, text: &str) -> Option<&'m String> {
    let t = text.trim().to_lowercase();
    map.entry_points
        .iter()
        .find(|e| e.to_lowercase() == t)
        .or_else(|| {
            let mut hits = map
                .entry_points
                .iter()
                .filter(|e| e.to_lowercase().contains(&t));
            let first = hits.next()?;
            hits.next().is_none().then_some(first)
        })
}

fn find_vehicle<'c>(
    catalogue: &'c GameCatalogue,
    record: &SessionVehicleRecord,
) -> Option<&'c VehicleItem> {
    let by_id = |id: &str| {
        catalogue
            .vehicles
            .iter()
            .find(|v| !id.is_empty() && v.id.eq_ignore_ascii_case(id))
    };
    by_id(&record.catalogue_id).or_else(|| {
        catalogue
            .vehicles
            .iter()
            .find(|v| v.full_name == record.name)
    })
}

/// The rules of the map change.
///
/// # Errors
/// See [`ChangeMapError`].
pub fn plan(
    inputs: &PlanInputs<'_>,
    map: &str,
    entry: Option<&str>,
    choice: VehicleChoice,
) -> Result<ChangeMapPlan, ChangeMapError> {
    let maps = find_maps(inputs.catalogue, map);
    let target = match maps.as_slice() {
        [] => return Err(ChangeMapError::UnknownMap(map.to_owned())),
        [one] => *one,
        many => {
            return Err(ChangeMapError::AmbiguousMap(
                map.to_owned(),
                many.iter().map(|m| m.friendly_name.clone()).collect(),
            ))
        }
    };
    let (entry_point, entry_defaulted) = match entry.filter(|e| !e.trim().is_empty()) {
        Some(e) => (
            find_entry(target, e)
                .cloned()
                .ok_or_else(|| ChangeMapError::UnknownEntry {
                    map: target.friendly_name.clone(),
                    entry: e.to_owned(),
                    available: target.entry_points.clone(),
                })?,
            false,
        ),
        None => (
            target.entry_points.first().cloned().unwrap_or_default(),
            true,
        ),
    };
    let carried = match (choice, inputs.active) {
        (VehicleChoice::Carry, Some(record)) => Some((
            find_vehicle(inputs.catalogue, record)
                .ok_or_else(|| ChangeMapError::VehicleUnavailable(record.name.clone()))?,
            record,
        )),
        _ => None,
    };
    let (date, seconds_of_day, weather) = inputs.clock.clone();
    Ok(ChangeMapPlan {
        map_id: target.id.clone(),
        map: target.friendly_name.clone(),
        entry_point,
        entry_defaulted,
        vehicle_key: carried.map_or_else(String::new, |(v, _)| inputs.catalogue.vehicle_key(v)),
        vehicle_name: carried.map_or_else(String::new, |(v, _)| v.full_name.clone()),
        no_vehicle: carried.is_none(),
        repaint: carried.map_or_else(String::new, |(_, r)| r.repaint.clone()),
        setvars: carried.map_or_else(Vec::new, |(_, r)| r.setvars.clone()),
        date,
        seconds_of_day,
        weather,
        dropped: inputs.others.clone(),
    })
}

/// Plans a change in the running session `world`.
///
/// # Errors
/// [`ChangeMapError::NotInDrive`] outside a loaded drive, otherwise as [`plan`].
pub fn plan_in_world(
    world: &World,
    map: &str,
    entry: Option<&str>,
    choice: VehicleChoice,
) -> Result<ChangeMapPlan, ChangeMapError> {
    let in_drive = world
        .get_resource::<State<AppState>>()
        .is_some_and(|s| *s.get() == AppState::InGame)
        && world
            .get_resource::<crate::in_game::types::SimulationLoadState>()
            .is_none_or(|l| l.is_complete);
    if !in_drive {
        return Err(ChangeMapError::NotInDrive);
    }
    let catalogue = world
        .get_resource::<GameCatalogue>()
        .ok_or(ChangeMapError::NoCatalogue)?;
    let inputs = inputs_from(
        catalogue,
        world.get_resource::<SessionVehicles>(),
        session_clock(
            world.get_resource::<SessionEnvironment>(),
            world.get_resource::<NewGameSetup>(),
        ),
    );
    plan(&inputs, map, entry, choice)
}

/// The session clock: date, time of day in seconds and the weather preset; the setup's start
/// when there is no session environment.
#[must_use]
pub fn session_clock(
    env: Option<&SessionEnvironment>,
    setup: Option<&NewGameSetup>,
) -> (CalendarDate, f64, String) {
    match env {
        Some(e) => (e.date(), e.seconds_of_day(), e.weather().to_owned()),
        None => (
            CalendarDate::clamped(
                setup.map_or(2026, |s| s.year),
                setup.map_or(6, |s| s.month),
                setup.map_or(29, |s| s.day),
            ),
            f64::from(setup.map_or(12, |s| s.hour) * 3600 + setup.map_or(0, |s| s.minute) * 60),
            setup.map_or_else(String::new, |s| s.weather_preset.clone()),
        ),
    }
}

/// The inputs of [`plan`] from the session's vehicles and clock.
#[must_use]
pub fn inputs_from<'a>(
    catalogue: &'a GameCatalogue,
    registry: Option<&'a SessionVehicles>,
    clock: (CalendarDate, f64, String),
) -> PlanInputs<'a> {
    let active = registry.and_then(|r| r.active()).filter(|r| r.is_alive());
    let active_id = active.map(|r| r.id);
    let others = registry
        .map(|r| {
            r.records()
                .iter()
                .filter(|v| v.is_alive() && Some(v.id) != active_id)
                .map(|v| v.name.clone())
                .collect()
        })
        .unwrap_or_default();
    PlanInputs {
        catalogue,
        active,
        others,
        clock,
    }
}

/// Enters the session again on the planned map: the setup is written and the state is set to the
/// session once more, which runs the teardown of the old session and the loading of the new one.
pub fn start(world: &mut World, plan: &ChangeMapPlan) {
    plan.apply_to(&mut world.resource_mut::<NewGameSetup>());
    // A drive started with `--map-path`, `--vehicle-path` or `--livery` (developers, scripted
    // runs) would otherwise load the same map again: the change overrides those choices.
    if let Some(mut args) = world.get_resource_mut::<crate::AppArgs>() {
        args.0.map_path = None;
        args.0.vehicle_path = None;
        args.0.livery = None;
    }
    world
        .resource_mut::<NextState<AppState>>()
        .set(AppState::InGame);
    if let Some(mut menu) = world.get_resource_mut::<super::VehicleMenu>() {
        menu.close();
    }
    tracing::info!(map = %plan.map, entry = %plan.entry_point, dropped = plan.dropped.len(), "changing the map");
}

#[cfg(test)]
mod tests;
