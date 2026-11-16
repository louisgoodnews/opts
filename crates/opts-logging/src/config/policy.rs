//! Format, colour, verbosity, rotation and retention choices of the logging configuration.

use serde::{Deserialize, Serialize};

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
