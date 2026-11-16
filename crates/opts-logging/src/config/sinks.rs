//! Console and file output options of the logging configuration.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::policy::{AnsiMode, ConsoleVerbosity, RetentionPolicy, RotationPolicy};

/// Options controlling terminal console output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsoleConfig {
    /// Whether console logging is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// ANSI color escapes behavior.
    #[serde(default)]
    pub ansi: AnsiMode,

    /// Output verbosity level.
    #[serde(default)]
    pub verbosity: ConsoleVerbosity,
}

fn default_true() -> bool {
    true
}

impl Default for ConsoleConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ansi: AnsiMode::Auto,
            verbosity: ConsoleVerbosity::Standard,
        }
    }
}

/// Options controlling disk file outputs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesConfig {
    /// Whether file logging is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Destination directory for log files.
    #[serde(default = "default_log_dir")]
    pub directory: PathBuf,

    /// Rolling file rotation policy.
    #[serde(default)]
    pub rotation: RotationPolicy,

    /// Historical log file retention policy.
    #[serde(default)]
    pub retention: RetentionPolicy,

    /// Whether to generate per-subsystem log files (`opts-core.log`, etc.).
    #[serde(default = "default_true")]
    pub per_subsystem: bool,

    /// Whether file logs should be emitted in JSON format.
    #[serde(default)]
    pub json: bool,
}

fn default_log_dir() -> PathBuf {
    if let Some(proj_dirs) = directories::ProjectDirs::from("org", "opts", "opts") {
        proj_dirs.data_local_dir().join("logs")
    } else {
        PathBuf::from(".opts-logs")
    }
}

impl Default for FilesConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            directory: default_log_dir(),
            rotation: RotationPolicy::Daily,
            retention: RetentionPolicy::Days(7),
            per_subsystem: true,
            json: false,
        }
    }
}
