//! The catalogue handle: opening, connection access and staleness.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OptionalExtension};

use crate::{schema, Result, Staleness, SCAN_VERSION};

pub(crate) fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0))
}

/// Handle to the catalogue database (thread-safe).
#[derive(Debug)]
pub struct Catalogue {
    path: PathBuf,
    conn: Mutex<Connection>,
}

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

    pub(crate) fn with<T>(&self, f: impl FnOnce(&Connection) -> rusqlite::Result<T>) -> Result<T> {
        // A panic in another thread cannot leave the connection half-written (SQLite is
        // transactional), so a poisoned lock is safe to reuse.
        let guard = self
            .conn
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Ok(f(&guard)?)
    }

    pub(crate) fn meta(&self, key: &str) -> Result<Option<String>> {
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
