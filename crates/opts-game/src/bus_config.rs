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

/// Why the file could not be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreProblem {
    /// The file exists but could not be read.
    Unreadable(String),
    /// The file is not valid JSON of the expected shape.
    Corrupt(String),
    /// The file was written by a newer version of the game; it is left as it is.
    Newer(u32),
}

impl fmt::Display for StoreProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreadable(e) => write!(f, "the configurations file cannot be read: {e}"),
            Self::Corrupt(e) => write!(f, "the configurations file is damaged: {e}"),
            Self::Newer(v) => write!(
                f,
                "the configurations file has version {v}, this game reads up to {FORMAT_VERSION}"
            ),
        }
    }
}

/// All saved configurations.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct BusConfigurations {
    /// File layout version (0 in files without one).
    #[serde(default)]
    pub version: u32,
    /// The configurations, in the order they were first saved.
    #[serde(default)]
    pub configurations: Vec<BusConfiguration>,
}

/// Whether two names are the same ignoring case (also of umlauts) and surrounding spaces.
fn same_name(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

/// A slug for an id: lower-case letters and digits, other runs become one `-`.
fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let out = out.trim_end_matches('-').to_string();
    if out.is_empty() {
        "configuration".to_string()
    } else {
        out
    }
}

impl BusConfigurations {
    /// Where the file is stored (beside the settings database), if a directory is known.
    #[must_use]
    pub fn path() -> Option<PathBuf> {
        GameSettings::database_path().map(|p| p.with_file_name(FILE_NAME))
    }

    /// Reads the store at `path`; a missing file is an empty store.
    ///
    /// # Errors
    /// [`StoreProblem`] when the file cannot be read, is damaged or is from a newer version.
    pub fn load_from(path: &Path) -> Result<Self, StoreProblem> {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(StoreProblem::Unreadable(e.to_string())),
        };
        let mut store: Self =
            serde_json::from_str(&text).map_err(|e| StoreProblem::Corrupt(e.to_string()))?;
        if store.version > FORMAT_VERSION {
            return Err(StoreProblem::Newer(store.version));
        }
        // Files before the version field (0) have the layout of version 1.
        store.version = FORMAT_VERSION;
        Ok(store)
    }

    /// Writes the store to `path` (through a temporary file, so a crash keeps the old one). A
    /// damaged file that is there is first kept as `<name>.broken`.
    ///
    /// # Errors
    /// The I/O error when the file cannot be written.
    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        if matches!(Self::load_from(path), Err(StoreProblem::Corrupt(_))) {
            let mut broken = path.as_os_str().to_owned();
            broken.push(".broken");
            std::fs::rename(path, broken)?;
        }
        let mut copy = self.clone();
        copy.version = FORMAT_VERSION;
        let json = serde_json::to_string_pretty(&copy).map_err(std::io::Error::other)?;
        let mut tmp = path.as_os_str().to_owned();
        tmp.push(".tmp");
        let tmp = PathBuf::from(tmp);
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, path)
    }

    /// The configuration with this id, else the one with this name (ignoring case).
    #[must_use]
    pub fn find(&self, key: &str) -> Option<&BusConfiguration> {
        self.configurations
            .iter()
            .find(|c| c.id == key)
            .or_else(|| self.configurations.iter().find(|c| same_name(&c.name, key)))
    }

    fn position(&self, key: &str) -> Option<usize> {
        let id = self.find(key)?.id.clone();
        self.configurations.iter().position(|c| c.id == id)
    }

    /// An id for a new configuration called `name`: its slug, numbered when taken.
    fn fresh_id(&self, name: &str) -> String {
        let base = slug(name);
        let taken = |id: &str| self.configurations.iter().any(|c| c.id == id);
        if !taken(&base) {
            return base;
        }
        (2..)
            .map(|n| format!("{base}-{n}"))
            .find(|id| !taken(id))
            .unwrap_or(base)
    }

    /// Saves `config`: a configuration with the same name (ignoring case) is replaced and keeps
    /// its id, creation time, notes and favourite mark; otherwise it is added. Returns the id
    /// and whether one was replaced.
    pub fn save(&mut self, mut config: BusConfiguration) -> (String, bool) {
        if let Some(at) = self
            .configurations
            .iter()
            .position(|c| same_name(&c.name, &config.name))
        {
            let old = &self.configurations[at];
            config.id = old.id.clone();
            config.created = old.created;
            if config.notes.is_empty() {
                config.notes = old.notes.clone();
            }
            config.favourite = old.favourite;
            let id = config.id.clone();
            self.configurations[at] = config;
            return (id, true);
        }
        config.id = self.fresh_id(&config.name);
        let id = config.id.clone();
        self.configurations.push(config);
        (id, false)
    }

    /// Removes a configuration; whether there was one.
    pub fn delete(&mut self, key: &str) -> bool {
        match self.position(key) {
            Some(at) => {
                self.configurations.remove(at);
                true
            }
            None => false,
        }
    }

    /// Renames a configuration. `false` when it is missing or the name is empty or taken by
    /// another one.
    pub fn rename(&mut self, key: &str, name: &str) -> bool {
        let name = name.trim();
        let Some(at) = self.position(key) else {
            return false;
        };
        let clash = self
            .configurations
            .iter()
            .enumerate()
            .any(|(i, c)| i != at && same_name(&c.name, name));
        if name.is_empty() || clash {
            return false;
        }
        self.configurations[at].name = name.to_string();
        true
    }

    /// Copies a configuration under `name`; the id of the copy, `None` when the original is
    /// missing or the name is empty or taken.
    pub fn duplicate(&mut self, key: &str, name: &str, now: i64) -> Option<String> {
        let name = name.trim();
        if name.is_empty() || self.find(name).is_some() {
            return None;
        }
        let mut copy = self.find(key)?.clone();
        copy.name = name.to_string();
        copy.favourite = false;
        copy.created = now;
        copy.last_used = now;
        copy.id = self.fresh_id(name);
        let id = copy.id.clone();
        self.configurations.push(copy);
        Some(id)
    }

    /// Sets the favourite mark; `false` when the configuration is missing.
    pub fn set_favourite(&mut self, key: &str, favourite: bool) -> bool {
        match self.position(key) {
            Some(at) => {
                self.configurations[at].favourite = favourite;
                true
            }
            None => false,
        }
    }

    /// Stamps a configuration as used now.
    pub fn touch(&mut self, key: &str, now: i64) {
        if let Some(at) = self.position(key) {
            self.configurations[at].last_used = now;
        }
    }
}

#[cfg(test)]
mod tests;
