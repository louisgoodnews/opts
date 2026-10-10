//! Configuration types and loading logic for `opts-logging`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::error::{LogError, LogResult};

/// Global logging configuration schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogConfig {
    /// Global default log level (e.g. "info", "debug", "warn", "error", "trace").
    #[serde(default = "default_global_level")]
    pub level: String,

    /// Per-target level overrides (e.g. "wgpu" -> "warn", "opts_sim" -> "trace").
    #[serde(default)]
    pub per_target: BTreeMap<String, String>,

    /// Console output options.
    #[serde(default)]
    pub console: ConsoleConfig,

    /// File logging options.
    #[serde(default)]
    pub files: FilesConfig,

    /// Log record format (Text or Json).
    #[serde(default)]
    pub format: LogFormat,
}

fn default_global_level() -> String {
    "info".to_string()
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: default_global_level(),
            per_target: BTreeMap::new(),
            console: ConsoleConfig::default(),
            files: FilesConfig::default(),
            format: LogFormat::Text,
        }
    }
}

impl LogConfig {
    /// Loads and parses configuration from a TOML file.
    pub fn from_file(path: impl AsRef<Path>) -> LogResult<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    /// Loads defaults and overrides from environment variables:
    /// - `OPTS_LOG_LEVEL` or `RUST_LOG` overrides global level
    /// - `OPTS_LOG_CONSOLE` overrides console enabled (true/false)
    /// - `OPTS_LOG_DIR` overrides the file logging directory
    /// - `OPTS_LOG_FORMAT` overrides format ("json" or "text")
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(lvl) = std::env::var("OPTS_LOG_LEVEL").or_else(|_| std::env::var("RUST_LOG")) {
            if !lvl.trim().is_empty() {
                config.level = lvl.trim().to_string();
            }
        }

        if let Ok(console_flag) = std::env::var("OPTS_LOG_CONSOLE") {
            if let Ok(b) = console_flag.trim().parse::<bool>() {
                config.console.enabled = b;
            }
        }

        if let Ok(dir_str) = std::env::var("OPTS_LOG_DIR") {
            let trimmed = dir_str.trim();
            if !trimmed.is_empty() {
                config.files.directory = PathBuf::from(trimmed);
            }
        }

        if let Ok(fmt_str) = std::env::var("OPTS_LOG_FORMAT") {
            match fmt_str.trim().to_lowercase().as_str() {
                "json" => config.format = LogFormat::Json,
                "text" => config.format = LogFormat::Text,
                _ => {}
            }
        }

        config
    }

    /// Validates the configuration to ensure invariants are upheld.
    pub fn validate(&self) -> LogResult<()> {
        if self.level.trim().is_empty() {
            return Err(LogError::InvalidConfig(
                "global log level cannot be empty".to_string(),
            ));
        }

        if self.files.enabled {
            if self.files.directory.as_os_str().is_empty() {
                return Err(LogError::InvalidConfig(
                    "log directory cannot be empty when file logging is enabled".to_string(),
                ));
            }

            // Validate rotation and retention compatibility
            if let (RotationPolicy::Daily, RetentionPolicy::Days(d)) =
                (&self.files.rotation, &self.files.retention)
            {
                if *d == 0 {
                    return Err(LogError::InvalidConfig(
                        "retention days must be greater than 0 for daily rotation".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }
}

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

/// Record formatting style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    /// Human-readable text format with formatted timestamps and fields.
    #[default]
    Text,
    /// Structured NDJSON (Newline Delimited JSON) for machine parsing.
    Json,
}

/// ANSI color behavior for console output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AnsiMode {
    /// Auto-detect based on whether stdout is an interactive terminal.
    #[default]
    Auto,
    /// Force ANSI colors enabled.
    Always,
    /// Disable all ANSI color sequences.
    Never,
}

/// Console formatting verbosity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ConsoleVerbosity {
    /// Minimal: single line per record without target and span info.
    Compact,
    /// Default: includes timestamps, level badges, target, message, and spans.
    #[default]
    Standard,
    /// Verbose: includes full module paths, source file, line numbers, and fields.
    Verbose,
}

/// File rotation policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RotationPolicy {
    /// Roll files once per day.
    #[default]
    Daily,
    /// Roll files once per hour.
    Hourly,
    /// Never roll; write continuously to single files.
    Never,
}

/// Retention pruning policy for old log files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RetentionPolicy {
    /// Keep logs newer than N days.
    Days(u32),
    /// Keep at most N most-recent log files per prefix.
    Count(u32),
    /// Never delete old logs.
    Forever,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self::Days(7)
    }
}
