//! The user's saved formations: one TOML file each in `<user data dir>/formations/`.
//!
//! ```toml
//! format = 1
//! name = "Three NF6D"
//! template = "Trains\\X10_112_IC-8.zug"   # optional: the preset it started from
//!
//! [[segment]]
//! vehicle = "Vehicles\\Tram_NF6D\\NF6D_A.bus"
//! orientation = "forward"                  # forward | backward
//! powered = true                           # optional
//! ```
//!
//! A segment names a `vehicle` file or a vehicle `group`. Files are written atomically (a temporary
//! file renamed over the target) and a file that cannot be read is listed with its error, never
//! deleted. Schema and rules: `docs/src/architecture/train-formation-editor.md` section 8.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use directories::ProjectDirs;
use opts_core::formats::trains::{
    FormationSegment, FormationSource, Orientation, TrainFormation, VehicleRef, MAX_CARS,
};
use serde::{Deserialize, Serialize};

use crate::save::container::{slug, valid_id};

/// The schema version this build reads and writes.
pub const FORMAT: u32 = 1;
/// Files larger than this are refused.
const MAX_FILE_BYTES: u64 = 1 << 20;
const EXT: &str = "toml";

/// Why a formation file could not be used.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StoreError {
    /// The id is not a formation id.
    #[error("`{0}` is not a formation id")]
    InvalidName(String),
    /// No such formation.
    #[error("no saved formation `{0}`")]
    NotFound(String),
    /// A file system error.
    #[error("{0}")]
    Io(String),
    /// The text is not a formation file.
    #[error("{0}")]
    Corrupt(String),
    /// Written by a newer version of OPTS.
    #[error("written by a newer OPTS (format {0}, this build reads {FORMAT})")]
    Newer(u32),
}

mod codec;
mod io;
pub use codec::{decode, encode};

/// One file of the store.
#[derive(Debug, Clone, PartialEq)]
pub struct SavedFormation {
    /// File stem; what the control protocol and the UI refer to.
    pub id: String,
    /// The formation, or why the file cannot be used.
    pub state: Result<TrainFormation, StoreError>,
    /// When the file was last written.
    pub modified: Option<SystemTime>,
}

/// The folder of saved formations.
#[derive(Debug, Clone)]
pub struct FormationStore {
    dir: PathBuf,
}

impl FormationStore {
    /// A store in `dir` (created on the first write).
    #[must_use]
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// `<user data dir>/formations`.
    #[must_use]
    pub fn default_dir() -> Option<PathBuf> {
        ProjectDirs::from("org", "opts", "opts").map(|d| d.data_dir().join("formations"))
    }

    /// The user's store, if a data directory is known.
    #[must_use]
    pub fn user() -> Option<Self> {
        Self::default_dir().map(Self::new)
    }

    /// The folder.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.{EXT}"))
    }

    fn check(id: &str) -> Result<(), StoreError> {
        if valid_id(id) {
            Ok(())
        } else {
            Err(StoreError::InvalidName(id.to_owned()))
        }
    }

    /// An id for a formation called `name` that no file uses yet (`name`, `name-2`, ...).
    #[must_use]
    pub fn unique_id(&self, name: &str) -> String {
        let base = slug(name);
        let mut id = base.clone();
        let mut n = 2;
        while self.path(&id).exists() {
            id = format!("{base}-{n}");
            n += 1;
        }
        id
    }
}

#[cfg(test)]
mod tests;
