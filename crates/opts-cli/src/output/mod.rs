//! Rendering of results and errors for the terminal.

pub mod human;
pub mod json;

use serde_json::Value;

use crate::protocol::RpcError;

/// How results are printed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    /// Readable text.
    Human,
    /// One JSON document per result (`--json`).
    Json,
    /// Nothing on success (`--quiet`); errors still go to stderr.
    Quiet,
}

/// Text to print on stdout for a successful result, or `None` for nothing.
#[must_use]
pub fn render_result(mode: OutputMode, method: &str, result: &Value) -> Option<String> {
    match mode {
        OutputMode::Human => Some(human::format_result(method, result)),
        OutputMode::Json => Some(json::format_result(result)),
        OutputMode::Quiet => None,
    }
}

/// Text to print on stderr for a failed call.
#[must_use]
pub fn render_error(mode: OutputMode, error: &RpcError) -> String {
    match mode {
        OutputMode::Json => json::format_error(error),
        OutputMode::Human | OutputMode::Quiet => human::format_error(error),
    }
}
