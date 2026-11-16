//! The reload messages that flow between the watcher, the validation and the applier.

use super::*;

/// A file changed (after debouncing) or a reload was requested explicitly.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct ReloadRequested {
    /// The file.
    pub path: PathBuf,
    /// Its kind.
    pub kind: ReloadKind,
    /// Sequence number, unique per request (answers of `reload.*` use it).
    pub seq: u64,
}

/// The file parsed successfully; the application may now replace the affected content.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct ReloadValidated {
    /// The file.
    pub path: PathBuf,
    /// Its kind.
    pub kind: ReloadKind,
    /// Sequence number of the request.
    pub seq: u64,
}

/// The file could not be used; nothing was changed.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct ReloadFailed {
    /// The file.
    pub path: PathBuf,
    /// Its kind.
    pub kind: ReloadKind,
    /// Sequence number of the request.
    pub seq: u64,
    /// Why.
    pub error: String,
}

/// The application replaced the content that depended on the file.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct ReloadApplied {
    /// The file.
    pub path: PathBuf,
    /// Its kind.
    pub kind: ReloadKind,
    /// Sequence number of the request.
    pub seq: u64,
    /// Human-readable list of what was rebuilt (`2 tiles`, `1 texture`, …).
    pub summary: String,
}

/// Asks the application to rebuild everything that is loaded (`reload.scene`).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReloadSceneRequested {
    /// Sequence number of the request.
    pub seq: u64,
}

/// Final outcome of one request, kept for `reload.*` replies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReloadOutcome {
    /// Request sequence number.
    pub seq: u64,
    /// The file (empty for scene reloads).
    pub path: PathBuf,
    /// `Ok(summary)` or `Err(error)`.
    pub result: Result<String, String>,
}
