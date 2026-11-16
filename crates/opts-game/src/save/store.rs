//! Files of the save list: where they live, atomic writes, the rotating backup, recovery.
//!
//! `<dir>/<id>.save.json` is the save, `<id>.save.json.bak` the previous version, and
//! `<id>.save.json.tmp` exists only while a write is in progress. A write never touches the main
//! file until the new text is complete on disk:
//!
//! 1. write the text to `.tmp` and `fsync` it;
//! 2. move the old main file to `.bak` (replacing the older backup);
//! 3. rename `.tmp` to the main file.
//!
//! A crash before step 3 leaves either the old main file or (between 2 and 3) only the backup,
//! which [`SaveStore::read`] falls back to.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;

use super::container::{slug, valid_id, SaveError, SaveFile, SaveMeta, MAX_FILE_BYTES};

const EXT: &str = "save.json";

/// A folder of saves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveStore {
    dir: PathBuf,
}

/// A save read from the folder.
#[derive(Debug, Clone, PartialEq)]
pub struct Loaded {
    /// The save.
    pub file: SaveFile,
    /// The main file was missing or damaged and the backup was used.
    pub from_backup: bool,
}

/// One line of the save list.
#[derive(Debug, Clone, PartialEq)]
pub struct SaveEntry {
    /// File stem; what the control protocol and the UI refer to.
    pub id: String,
    /// The save, or why it cannot be read.
    pub state: Result<Loaded, SaveError>,
}

impl SaveEntry {
    /// The metadata when the file could be read.
    #[must_use]
    pub fn meta(&self) -> Option<&SaveMeta> {
        self.state.as_ref().ok().map(|l| &l.file.meta)
    }
}

impl SaveStore {
    /// A store in `dir` (created on the first write).
    #[must_use]
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// The folder of the user's saves: `<user data dir>/saves`.
    #[must_use]
    pub fn default_dir() -> Option<PathBuf> {
        ProjectDirs::from("org", "opts", "opts").map(|d| d.data_dir().join("saves"))
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

    fn main_path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.{EXT}"))
    }

    fn with_suffix(&self, id: &str, suffix: &str) -> PathBuf {
        self.dir.join(format!("{id}.{EXT}.{suffix}"))
    }

    fn check(id: &str) -> Result<(), SaveError> {
        if valid_id(id) {
            Ok(())
        } else {
            Err(SaveError::InvalidName(format!("`{id}` is not a save id")))
        }
    }

    /// An id for a new save called `name` that no file uses yet.
    #[must_use]
    pub fn unique_id(&self, name: &str) -> String {
        let base = slug(name);
        let mut id = base.clone();
        let mut n = 2;
        while self.main_path(&id).exists() || self.with_suffix(&id, "bak").exists() {
            id = format!("{base}-{n}");
            n += 1;
        }
        id
    }

}

mod read_write;
mod listing;


#[cfg(test)]
mod tests;
