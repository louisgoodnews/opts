//! The wizard's view of the drive history: remembered choices, recents, favourites, and the write
//! when a drive starts.

use std::path::PathBuf;

use bevy::prelude::*;

use super::planner::Note;
use super::types::{GameCatalogue, NewGameSetup};
use crate::drive_history::{DriveHistory, DriveRecord};

/// The drive history as the wizard keeps it during a session.
#[derive(Resource, Debug, Clone)]
pub struct PlannerHistory {
    /// The history (file `drive_history.json` beside the settings database).
    pub history: DriveHistory,
    /// Where it is saved; `None` keeps it in memory only (tests, no config directory).
    pub path: Option<PathBuf>,
    /// Whether the last drive's choices were already applied in this session.
    pub applied: bool,
}

impl Default for PlannerHistory {
    fn default() -> Self {
        let path = DriveHistory::path();
        let history = path
            .as_deref()
            .map(DriveHistory::load_from)
            .unwrap_or_default();
        Self {
            history,
            path,
            applied: false,
        }
    }
}

impl PlannerHistory {
    /// A history stored at `path` (or only in memory).
    pub fn at(path: Option<PathBuf>) -> Self {
        let history = path
            .as_deref()
            .map(DriveHistory::load_from)
            .unwrap_or_default();
        Self {
            history,
            path,
            applied: false,
        }
    }

    /// Writes the history.
    ///
    /// # Errors
    /// The I/O error of the write.
    pub fn save(&self) -> std::io::Result<()> {
        match &self.path {
            Some(p) => self.history.save_to(p),
            None => Ok(()),
        }
    }

    /// Records the drive that starts now and saves.
    ///
    /// # Errors
    /// The I/O error of the write (the record is kept in memory either way).
    pub fn record_drive(&mut self, setup: &NewGameSetup, now: i64) -> std::io::Result<()> {
        self.history.record(DriveRecord::from_setup(setup, now));
        self.save()
    }

    /// Stars or un-stars a map (folder name) or a vehicle (id); saves.
    ///
    /// # Errors
    /// The I/O error of the write.
    pub fn set_favourite(&mut self, vehicle: bool, id: &str, on: bool) -> std::io::Result<()> {
        let list = if vehicle {
            &mut self.history.favourite_vehicles
        } else {
            &mut self.history.favourite_maps
        };
        list.retain(|f| f != id);
        if on {
            list.push(id.to_string());
        }
        self.save()
    }

    /// Applies the last drive's choices once per session. Choices whose map or vehicle is gone
    /// are skipped and reported as notes; nothing happens while the catalogue is still empty.
    pub fn apply_last(&mut self, setup: &mut NewGameSetup, catalogue: &GameCatalogue) -> Vec<Note> {
        if self.applied || catalogue.maps.is_empty() || catalogue.vehicles.is_empty() {
            return Vec::new();
        }
        self.applied = true;
        let Some(rec) = self.history.last().cloned() else {
            return Vec::new();
        };
        let mut notes = Vec::new();
        match catalogue.map_by_name(&rec.map) {
            Some(map) => {
                setup.selected_map.clone_from(&rec.map);
                if map.entry_points.contains(&rec.entry_point) {
                    setup.selected_entry_point.clone_from(&rec.entry_point);
                } else if let Some(first) = map.entry_points.first() {
                    setup.selected_entry_point.clone_from(first);
                }
            }
            None => notes.push(Note::MapMissing),
        }
        match catalogue.find_vehicle(&rec.vehicle) {
            Some(v) => {
                setup.selected_vehicle = catalogue.vehicle_key(v);
                setup.selected_livery.clone_from(&rec.livery);
                setup.custom_setvars = rec.setvars.iter().cloned().collect();
            }
            None => notes.push(Note::VehicleMissing),
        }
        setup.weather_preset.clone_from(&rec.weather_preset);
        (setup.year, setup.month, setup.day) = (rec.year, rec.month, rec.day);
        (setup.hour, setup.minute) = (rec.hour, rec.minute);
        notes
    }

    /// The remembered entry point of a map (the newest drive on it), if any.
    pub fn entry_point_for(&self, map: &str) -> Option<&str> {
        self.history
            .recents
            .iter()
            .find(|r| r.map == map)
            .map(|r| r.entry_point.as_str())
    }
}

#[cfg(test)]
mod tests;
