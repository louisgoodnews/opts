//! Catalogue errors and the crate result alias.

use std::path::PathBuf;

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
