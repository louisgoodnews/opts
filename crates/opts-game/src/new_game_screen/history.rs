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
mod tests {
    use super::*;
    use crate::new_game_screen::types::{MapItem, VehicleItem};

    fn cat() -> GameCatalogue {
        GameCatalogue {
            maps: vec![MapItem {
                id: "b".into(),
                name: "b".into(),
                friendly_name: "Berlin".into(),
                description: String::new(),
                tile_count: 1,
                standard_depot: String::new(),
                years: String::new(),
                entry_points: vec!["A".into(), "B".into()],
            }],
            vehicles: vec![VehicleItem::named("v", "MAN", "SD77")],
        }
    }

    fn rec(map: &str, vehicle: &str) -> DriveRecord {
        DriveRecord {
            map: map.into(),
            entry_point: "B".into(),
            vehicle: vehicle.into(),
            no_vehicle: false,
            livery: "Beige".into(),
            weather_preset: "Rainy Afternoon".into(),
            year: 1990,
            month: 5,
            day: 6,
            hour: 18,
            minute: 0,
            setvars: vec![("x".into(), 1)],
            last_driven: 5,
        }
    }

    #[test]
    fn the_last_drive_is_applied_once_per_session() {
        let mut h = PlannerHistory::at(None);
        h.history.record(rec("Berlin", "MAN SD77"));
        let mut s = NewGameSetup::default();
        let notes = h.apply_last(&mut s, &cat());
        assert!(notes.is_empty());
        assert_eq!(s.selected_map, "Berlin");
        assert_eq!(s.selected_entry_point, "B");
        assert_eq!(s.weather_preset, "Rainy Afternoon");
        assert_eq!((s.hour, s.minute), (18, 0));
        s.hour = 3;
        assert!(h.apply_last(&mut s, &cat()).is_empty());
        assert_eq!(s.hour, 3, "a second call does not overwrite later choices");
    }

    #[test]
    fn a_vanished_map_or_vehicle_becomes_a_note_and_keeps_the_default() {
        let mut h = PlannerHistory::at(None);
        h.history.record(rec("Gone", "Gone too"));
        let mut s = NewGameSetup {
            selected_map: "Berlin".into(),
            ..NewGameSetup::default()
        };
        let notes = h.apply_last(&mut s, &cat());
        assert_eq!(notes, vec![Note::MapMissing, Note::VehicleMissing]);
        assert_eq!(s.selected_map, "Berlin");
    }

    #[test]
    fn nothing_is_consumed_while_the_catalogue_is_empty() {
        let mut h = PlannerHistory::at(None);
        h.history.record(rec("Berlin", "MAN SD77"));
        let mut s = NewGameSetup::default();
        assert!(h.apply_last(&mut s, &GameCatalogue::default()).is_empty());
        assert!(!h.applied);
    }

    #[test]
    fn a_drive_is_recorded_and_written_to_the_file() {
        let dir = std::env::temp_dir().join(format!("opts_hist_{}", std::process::id()));
        let path = dir.join("drive_history.json");
        let _ = std::fs::remove_dir_all(&dir);
        let mut h = PlannerHistory::at(Some(path.clone()));
        let s = NewGameSetup {
            selected_map: "Berlin".into(),
            hour: 7,
            ..NewGameSetup::default()
        };
        h.record_drive(&s, 42).expect("write");
        h.set_favourite(false, "b", true).expect("write");
        let back = DriveHistory::load_from(&path);
        assert_eq!(
            back.last().map(|r| (r.map.as_str(), r.hour, r.last_driven)),
            Some(("Berlin", 7, 42))
        );
        assert_eq!(back.favourite_maps, vec!["b".to_string()]);
        h.set_favourite(false, "b", false).expect("write");
        assert!(DriveHistory::load_from(&path).favourite_maps.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
