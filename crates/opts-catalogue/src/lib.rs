//! # OPTS Catalogue (`opts-catalogue`)
//!
//! A SQLite cache of an OMSI 2 installation: every directory and file, the discovered content
//! entities (vehicles, maps, scenery objects, …), their display metadata and their references.
//! Menus and loaders query it instead of walking 600,000+ files on every launch.
//!
//! Design: `docs/src/architecture/catalogue-schema.md` (schema, staleness) and
//! `docs/src/architecture/playability-rules.md` (which vehicles are player-selectable).
//!
//! Staleness: the catalogue is stale when it was never completed, was built for a different
//! root, or is older than the caller's maximum age (24 h in the game). A stale catalogue is
//! rebuilt by [`discover`], which writes inside one transaction so readers keep seeing the
//! previous snapshot until the new one commits.

#![warn(missing_docs)]

pub mod extract;
pub mod scan;
pub mod schema;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension};

/// Catalogue errors.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// SQLite failure.
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    /// The OMSI root does not exist or is not a directory.
    #[error("OMSI root not found: {0}")]
    RootMissing(PathBuf),
    /// The database was written by a newer version of the application.
    #[error("catalogue schema version {found} is newer than supported version {supported}")]
    NewerSchema {
        /// Version found in the database.
        found: i64,
        /// Highest version this build understands.
        supported: i64,
    },
}

/// Result alias.
pub type Result<T> = std::result::Result<T, Error>;

/// Version of the scan logic. Bumped when what a scan records changes (new reference kinds,
/// rerouting) so that databases from older builds are rebuilt even if they are recent.
pub const SCAN_VERSION: i64 = 4;

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0))
}

/// Why (or whether) the catalogue needs a rescan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Staleness {
    /// No completed scan exists.
    Missing,
    /// The catalogue was built for a different OMSI root.
    RootChanged,
    /// The last scan is older than the allowed age.
    Expired {
        /// Age of the last scan.
        age_secs: i64,
    },
    /// The catalogue is current.
    Fresh {
        /// Age of the last scan.
        age_secs: i64,
    },
}

impl Staleness {
    /// Whether a rescan is required.
    #[must_use]
    pub fn needs_scan(self) -> bool {
        !matches!(self, Staleness::Fresh { .. })
    }
}

/// Scan phases reported to a progress callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Walking the filesystem.
    Walking,
    /// Detecting entities and reading their metadata.
    Detecting,
    /// Writing to SQLite.
    Writing,
}

/// Counts from a finished scan.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Summary {
    /// Files recorded.
    pub files: usize,
    /// Directories recorded.
    pub directories: usize,
    /// Entities recorded.
    pub entities: usize,
    /// References recorded.
    pub references: usize,
    /// References whose target does not exist anywhere in the installation.
    pub unresolved: usize,
    /// References whose path was wrong but whose file name exists elsewhere (rerouted).
    pub rerouted: usize,
}

/// A map row for menus.
#[derive(Debug, Clone, PartialEq)]
pub struct MapRecord {
    /// Entity id.
    pub entity_id: i64,
    /// Folder name under `maps/` (stable id).
    pub folder: String,
    /// Display name.
    pub friendly_name: String,
    /// Description.
    pub description: String,
    /// Number of tiles listed in `global.cfg`.
    pub tile_count: i64,
    /// Default depot.
    pub standard_depot: String,
    /// Year range text.
    pub years: String,
    /// Entry point names.
    pub entry_points: Vec<String>,
}

/// A vehicle row for menus.
#[derive(Debug, Clone, PartialEq)]
pub struct VehicleRecord {
    /// Entity id.
    pub entity_id: i64,
    /// `.bus` file stem (stable id).
    pub id: String,
    /// Marker path relative to the OMSI root.
    pub marker_file: String,
    /// Manufacturer.
    pub manufacturer: String,
    /// Model.
    pub model: String,
    /// Default livery.
    pub livery: String,
    /// Description.
    pub description: String,
    /// Curb mass in tonnes.
    pub mass_tonnes: Option<f64>,
    /// Vehicle class (`road`, `rail`, `aircraft`) when the content says it.
    pub class: Option<String>,
}

/// Handle to the catalogue database (thread-safe).
#[derive(Debug)]
pub struct Catalogue {
    path: PathBuf,
    conn: Mutex<Connection>,
}

mod lookup;

mod queries;

impl Catalogue {
    /// Opens (creating and migrating if needed) the catalogue at `db_path`.
    ///
    /// # Errors
    /// Fails if the file cannot be opened or was written by a newer schema.
    pub fn open(db_path: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = Connection::open(db_path)?;
        schema::prepare(&conn)?;
        Ok(Self {
            path: db_path.to_path_buf(),
            conn: Mutex::new(conn),
        })
    }

    /// Opens an existing catalogue for reading only: no file is created, no schema is migrated
    /// and nothing can be written. Several processes may hold it open at once (the scenery
    /// benchmark's workers, `docs/src/architecture/scenery-benchmark.md`, Q-SB-08).
    ///
    /// # Errors
    /// Fails if the file does not exist or cannot be opened.
    pub fn open_read_only(db_path: &Path) -> Result<Self> {
        let flags =
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let conn = Connection::open_with_flags(db_path, flags)?;
        Ok(Self {
            path: db_path.to_path_buf(),
            conn: Mutex::new(conn),
        })
    }

    /// Path of the database file.
    #[must_use]
    pub fn db_path(&self) -> &Path {
        &self.path
    }

    fn with<T>(&self, f: impl FnOnce(&Connection) -> rusqlite::Result<T>) -> Result<T> {
        // A panic in another thread cannot leave the connection half-written (SQLite is
        // transactional), so a poisoned lock is safe to reuse.
        let guard = self
            .conn
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Ok(f(&guard)?)
    }

    fn meta(&self, key: &str) -> Result<Option<String>> {
        self.with(|c| {
            c.query_row("SELECT value FROM meta WHERE key=?1", [key], |r| r.get(0))
                .optional()
        })
    }

    /// OMSI root recorded by the last scan.
    ///
    /// # Errors
    /// SQLite failures.
    pub fn root(&self) -> Result<Option<PathBuf>> {
        Ok(self.meta("omsi_root")?.map(PathBuf::from))
    }

    /// Compares the last completed scan against `root` and `max_age`.
    ///
    /// # Errors
    /// SQLite failures.
    pub fn staleness(&self, root: &Path, max_age: Duration) -> Result<Staleness> {
        let Some(done) = self
            .meta("last_scan_completed_at")?
            .and_then(|v| v.parse::<i64>().ok())
        else {
            return Ok(Staleness::Missing);
        };
        if self
            .meta("scan_version")?
            .and_then(|v| v.parse::<i64>().ok())
            != Some(SCAN_VERSION)
        {
            return Ok(Staleness::Missing);
        }
        if self.meta("omsi_root")?.as_deref() != Some(&root.to_string_lossy()) {
            return Ok(Staleness::RootChanged);
        }
        let age = (now_secs() - done).max(0);
        let limit = i64::try_from(max_age.as_secs()).unwrap_or(i64::MAX);
        Ok(if age > limit {
            Staleness::Expired { age_secs: age }
        } else {
            Staleness::Fresh { age_secs: age }
        })
    }
}

/// The catalogue by kind (see [`Catalogue::kind_counts`]); `other` is everything that is not one
/// of the four named kinds (trailers and AI vehicles, fonts, humans, weather, ...).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KindCounts {
    /// Files counted by the last scan.
    pub files: i64,
    /// Entries of every kind.
    pub entities: i64,
    /// Maps (as listed on Home and in the wizard).
    pub maps: i64,
    /// Player-selectable vehicles (as listed on Home and in the wizard).
    pub vehicles: i64,
    /// Scenery objects (`.sco`).
    pub scenery_objects: i64,
    /// Splines (`.sli`).
    pub splines: i64,
    /// All other entries.
    pub other: i64,
}

mod relocation;

mod discover;
pub use discover::discover;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod relocation_tests;
