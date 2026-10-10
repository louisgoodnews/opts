//! Error types for the `opts-logging` crate.

use std::path::PathBuf;
use thiserror::Error;

/// Result alias for operations in `opts-logging`.
pub type LogResult<T> = Result<T, LogError>;

/// Errors that can occur during logger configuration, initialization, or runtime.
#[derive(Debug, Error)]
pub enum LogError {
    /// Logging has already been initialized globally.
    #[error("logging has already been initialized globally")]
    AlreadyInitialized,

    /// Configuration error (e.g. invalid file path or conflicting settings).
    #[error("invalid logging configuration: {0}")]
    InvalidConfig(String),

    /// Standard I/O error occurred during file operations.
    #[error("I/O error during log operation: {0}")]
    Io(#[from] std::io::Error),

    /// Error parsing TOML configuration file.
    #[error("failed to parse TOML configuration: {0}")]
    Toml(#[from] toml::de::Error),

    /// Error setting the global tracing subscriber.
    #[error("failed to set global tracing subscriber: {0}")]
    SetSubscriber(String),

    /// Error resolving the platform log directory.
    #[error("unable to resolve platform log directory: {0}")]
    DirectoryResolution(String),

    /// Path not found or directory creation failure.
    #[error("failed to create log directory at {path}: {source}")]
    DirectoryCreation {
        /// Target path that could not be created.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// Filter directive parsing failed.
    #[error("failed to parse filter directive: {0}")]
    FilterParse(String),
}
