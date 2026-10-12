//! Remembered drives: the last choices, recent drives and favourites.
//!
//! Stored as `drive_history.json` next to `settings.db`, independent of the settings registry.
//! The *Plan a drive* wizard writes it when a drive starts; *Home* reads it for the *Continue*
//! card and the recent drives (`docs/src/roadmap/phase-9/flow.md` §3.2–3.3).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::new_game_screen::types::NewGameSetup;
use crate::settings::GameSettings;

/// One remembered choice of map, vehicle and conditions.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DriveRecord {
    /// Map folder name.
    pub map: String,
    /// Start point name.
    pub entry_point: String,
    /// Vehicle name as shown in the vehicle list.
    pub vehicle: String,
    /// Chosen repaint, empty for the default.
    pub livery: String,
    /// Weather preset name.
    pub weather_preset: String,
    /// Date and time of the drive (the simulated clock at the start).
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
    pub setvars: Vec<(String, i32)>,
    /// Unix time (seconds) of the last drive with these choices.
    pub last_driven: i64,
}

impl DriveRecord {
    /// The record of the choices in `setup`, stamped with `now` (unix seconds).
    #[must_use]
    pub fn from_setup(setup: &NewGameSetup, now: i64) -> Self {
        let mut setvars: Vec<_> = setup
            .custom_setvars
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        setvars.sort();
        Self {
            map: setup.selected_map.clone(),
            entry_point: setup.selected_entry_point.clone(),
            vehicle: setup.selected_vehicle.clone(),
            livery: setup.selected_livery.clone(),
            weather_preset: setup.weather_preset.clone(),
            year: setup.year,
            month: setup.month,
            day: setup.day,
            hour: setup.hour,
            minute: setup.minute,
            setvars,
            last_driven: now,
        }
    }

    /// Writes the remembered choices into `setup` (other fields keep their values).
    pub fn apply_to(&self, setup: &mut NewGameSetup) {
        setup.selected_map = self.map.clone();
        setup.selected_entry_point = self.entry_point.clone();
        setup.selected_vehicle = self.vehicle.clone();
        setup.selected_livery = self.livery.clone();
        setup.weather_preset = self.weather_preset.clone();
        setup.year = self.year;
        setup.month = self.month;
        setup.day = self.day;
        setup.hour = self.hour;
        setup.minute = self.minute;
        setup.custom_setvars = self.setvars.iter().cloned().collect();
    }

    /// Whether two records describe the same choices (the drive time stamp is ignored).
    fn same_choices(&self, other: &Self) -> bool {
        Self {
            last_driven: 0,
            ..self.clone()
        } == Self {
            last_driven: 0,
            ..other.clone()
        }
    }
}

/// Recent drives and favourites.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DriveHistory {
    /// Recent drives, most recent first, at most [`Self::MAX_RECENTS`].
    #[serde(default)]
    pub recents: Vec<DriveRecord>,
    /// Favourite map folder names.
    #[serde(default)]
    pub favourite_maps: Vec<String>,
    /// Favourite vehicle names.
    #[serde(default)]
    pub favourite_vehicles: Vec<String>,
}

impl DriveHistory {
    /// The number of recent drives kept.
    pub const MAX_RECENTS: usize = 6;

    /// Where the history is stored (beside the settings database), if a directory is known.
    #[must_use]
    pub fn path() -> Option<PathBuf> {
        GameSettings::database_path().map(|p| p.with_file_name("drive_history.json"))
    }

    /// Reads the history; a missing or unreadable file gives an empty history.
    #[must_use]
    pub fn load() -> Self {
        Self::path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Writes the history.
    ///
    /// # Errors
    /// Returns the I/O error when the file cannot be written.
    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = Self::path() else {
            return Ok(());
        };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(path, json)
    }

    /// Records a drive: the same choices move to the front instead of being listed twice.
    pub fn record(&mut self, drive: DriveRecord) {
        self.recents.retain(|r| !r.same_choices(&drive));
        self.recents.insert(0, drive);
        self.recents.truncate(Self::MAX_RECENTS);
    }

    /// The most recent drive, if any.
    #[must_use]
    pub fn last(&self) -> Option<&DriveRecord> {
        self.recents.first()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(map: &str, t: i64) -> DriveRecord {
        DriveRecord {
            map: map.into(),
            last_driven: t,
            ..Default::default()
        }
    }

    #[test]
    fn the_same_choices_move_to_the_front_and_are_listed_once() {
        let mut h = DriveHistory::default();
        h.record(rec("A", 1));
        h.record(rec("B", 2));
        h.record(rec("A", 3));
        assert_eq!(h.recents.len(), 2);
        assert_eq!(
            h.last().map(|r| (r.map.as_str(), r.last_driven)),
            Some(("A", 3))
        );
    }

    #[test]
    fn only_the_newest_drives_are_kept() {
        let mut h = DriveHistory::default();
        for i in 0..10 {
            h.record(rec(&format!("m{i}"), i));
        }
        assert_eq!(h.recents.len(), DriveHistory::MAX_RECENTS);
        assert_eq!(h.recents[0].map, "m9");
    }

    #[test]
    fn a_setup_survives_a_round_trip_through_a_record() {
        let mut setup = NewGameSetup {
            selected_map: "Grundorf".into(),
            hour: 17,
            ..NewGameSetup::default()
        };
        setup.custom_setvars.insert("Klima".into(), 2);
        let record = DriveRecord::from_setup(&setup, 42);
        let json = serde_json::to_string(&record).expect("serialises");
        let back: DriveRecord = serde_json::from_str(&json).expect("parses");
        let mut other = NewGameSetup::default();
        back.apply_to(&mut other);
        assert_eq!(other.selected_map, "Grundorf");
        assert_eq!(other.hour, 17);
        assert_eq!(other.custom_setvars.get("Klima"), Some(&2));
    }
}
