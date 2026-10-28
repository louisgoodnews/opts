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

impl Problem {
    /// A short stable name for protocol clients.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::VehicleMissing { .. } => "vehicle_missing",
            Self::RepaintMissing { .. } => "repaint_missing",
            Self::SetvarUnknown { .. } => "setvar_unknown",
            Self::SetvarRejected { .. } => "setvar_rejected",
        }
    }

    /// Whether the configuration can still be loaded (with this part left out).
    #[must_use]
    pub fn is_fatal(&self) -> bool {
        matches!(self, Self::VehicleMissing { .. })
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VehicleMissing { vehicle } => {
                write!(f, "the vehicle `{vehicle}` is not installed")
            }
            Self::RepaintMissing { repaint } => write!(
                f,
                "the repaint `{repaint}` is not offered by the vehicle; the default is used"
            ),
            Self::SetvarUnknown { name } => {
                write!(f, "the vehicle has no option `{name}`; it is not applied")
            }
            Self::SetvarRejected { name, value } => write!(
                f,
                "the value {value} is not accepted for the option `{name}`; it is not applied"
            ),
        }
    }
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

impl BusConfiguration {
    /// The configuration of the choices in `setup` named `name`, stamped with `now`; `None` when
    /// no vehicle is chosen or it is not in the catalogue.
    #[must_use]
    pub fn from_setup(
        name: &str,
        catalogue: &GameCatalogue,
        setup: &NewGameSetup,
        now: i64,
    ) -> Option<Self> {
        if setup.no_vehicle {
            return None;
        }
        let vehicle = catalogue.find_vehicle(&setup.selected_vehicle)?;
        Some(Self::new(
            name,
            &vehicle.id,
            &setup.selected_livery,
            setup
                .custom_setvars
                .iter()
                .map(|(k, v)| (k.clone(), *v))
                .collect(),
            now,
        ))
    }

    /// A configuration with its options sorted; the id is given when it is added to a store.
    #[must_use]
    pub fn new(
        name: &str,
        vehicle: &str,
        repaint: &str,
        mut setvars: Vec<(String, i32)>,
        now: i64,
    ) -> Self {
        setvars.sort();
        Self {
            id: String::new(),
            name: name.trim().to_string(),
            vehicle: vehicle.to_string(),
            repaint: repaint.to_string(),
            setvars,
            notes: String::new(),
            favourite: false,
            created: now,
            last_used: now,
        }
    }

    /// Checks the configuration against the installation: `repaints` gives the repaints and
    /// declared variables of a catalogue vehicle (the wizard's own discovery, or a fixed answer
    /// in tests).
    #[must_use]
    pub fn check(
        &self,
        catalogue: &GameCatalogue,
        repaints: impl Fn(&VehicleItem) -> VehicleRepaints,
    ) -> Checked {
        let mut out = Checked {
            repaint: self.repaint.clone(),
            setvars: Vec::new(),
            problems: Vec::new(),
        };
        let Some(vehicle) = catalogue.vehicles.iter().find(|v| v.id == self.vehicle) else {
            out.problems.push(Problem::VehicleMissing {
                vehicle: self.vehicle.clone(),
            });
            return out;
        };
        let reps = repaints(vehicle);
        if !self.repaint.is_empty() && !reps.names.iter().any(|n| same_name(n, &self.repaint)) {
            out.problems.push(Problem::RepaintMissing {
                repaint: self.repaint.clone(),
            });
            out.repaint.clear();
        }
        let options = catalogue_of(&reps);
        for (name, value) in &self.setvars {
            match options.get(name) {
                None => out
                    .problems
                    .push(Problem::SetvarUnknown { name: name.clone() }),
                Some(e) if e.accepts(f64::from(*value)) => {
                    out.setvars.push((e.name.clone(), *value));
                }
                Some(_) => out.problems.push(Problem::SetvarRejected {
                    name: name.clone(),
                    value: *value,
                }),
            }
        }
        out
    }

    /// Writes the checked choices into `setup`: the vehicle, the repaint and the options (other
    /// fields keep their values). `None` (nothing written) when the vehicle is missing.
    pub fn apply_to(
        &self,
        catalogue: &GameCatalogue,
        checked: &Checked,
        setup: &mut NewGameSetup,
    ) -> Option<()> {
        let vehicle = catalogue.vehicles.iter().find(|v| v.id == self.vehicle)?;
        if !checked.usable() {
            return None;
        }
        setup.no_vehicle = false;
        setup.selected_vehicle = catalogue.vehicle_key(vehicle);
        setup.selected_livery = checked.repaint.clone();
        setup.custom_setvars = checked.setvars.iter().cloned().collect();
        Some(())
    }
}

mod store;
use store::same_name;
pub use store::{BusConfigurations, StoreProblem};

#[cfg(test)]
mod tests;
