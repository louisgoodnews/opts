//! Bus configurations (M-0077): a vehicle, a repaint and equipment options saved under a name and
//! loaded again in one step.
//!
//! Stored as `bus_configurations.json` next to `drive_history.json` (and `settings.db`). The file
//! is versioned ([`FORMAT_VERSION`]); a file written by a newer version is left alone and reported,
//! a corrupt one is kept as `.broken` before the next save replaces it. A configuration is checked
//! when it is loaded ([`BusConfiguration::check`]): a vehicle that is gone makes it unusable, a
//! repaint that is gone falls back to the vehicle's default, an equipment value the vehicle's
//! catalogue (`opts_core::setvars`) no longer accepts is reported and not applied. Nothing here
//! panics on a bad file.

use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::new_game_screen::repaints::VehicleRepaints;
use crate::new_game_screen::setvars_discovery::catalogue_of;
use crate::new_game_screen::types::{GameCatalogue, NewGameSetup, VehicleItem};
use crate::settings::GameSettings;

/// The version of the file layout this build writes and reads.
pub const FORMAT_VERSION: u32 = 1;

/// File name beside the settings database.
pub const FILE_NAME: &str = "bus_configurations.json";

/// A named choice of vehicle, repaint and equipment options.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct BusConfiguration {
    /// Stable identifier (a slug of the first name; unique in the file).
    pub id: String,
    /// The name the user gave it.
    pub name: String,
    /// Catalogue id of the vehicle (`VehicleItem::id`).
    pub vehicle: String,
    /// Repaint name; empty for the vehicle's default.
    #[serde(default)]
    pub repaint: String,
    /// The user's changes of the equipment options, sorted by name.
    #[serde(default)]
    pub setvars: Vec<(String, i32)>,
    /// Free text.
    #[serde(default)]
    pub notes: String,
    /// Marked as favourite.
    #[serde(default)]
    pub favourite: bool,
    /// Unix time (seconds) of the first save.
    #[serde(default)]
    pub created: i64,
    /// Unix time (seconds) of the last load or save.
    #[serde(default)]
    pub last_used: i64,
}

/// What is wrong with a configuration against the installation it is loaded in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// The vehicle is not installed (any more): the configuration cannot be loaded.
    VehicleMissing {
        /// The catalogue id that was saved.
        vehicle: String,
    },
    /// The repaint is not offered by the vehicle (any more): the default repaint is used.
    RepaintMissing {
        /// The saved repaint.
        repaint: String,
    },
    /// The vehicle does not offer this option (any more): not applied.
    SetvarUnknown {
        /// The saved option.
        name: String,
    },
    /// The option's value is not accepted (any more): not applied.
    SetvarRejected {
        /// The saved option.
        name: String,
        /// The saved value.
        value: i32,
    },
}


/// A configuration as it can be applied: the repaint and options that survived the check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checked {
    /// The repaint to use (empty: the default).
    pub repaint: String,
    /// The options to apply.
    pub setvars: Vec<(String, i32)>,
    /// Everything that was left out or makes the configuration unusable.
    pub problems: Vec<Problem>,
}

impl Checked {
    /// Whether the configuration can be loaded at all.
    #[must_use]
    pub fn usable(&self) -> bool {
        !self.problems.iter().any(Problem::is_fatal)
    }
}

mod configuration;
mod problem;


mod store;
use store::same_name;
pub use store::{BusConfigurations, StoreProblem};

#[cfg(test)]
mod tests;
