//! The types of a restore: its errors, the pending state, the outcome and the started report.

use bevy::prelude::*;

use super::super::container::SaveError;
use super::super::registry::{Issue, RestorePlan};

/// How long a restore waits for its vehicles before it settles without them (seconds).
pub const SETTLE_TIMEOUT_S: f32 = 120.0;
/// Frames without a spawning vehicle before the saved vehicle state is applied.
pub const QUIET_FRAMES: u32 = 10;

/// Why a load was refused or failed.
#[derive(Debug, Clone, PartialEq)]
pub enum RestoreError {
    /// The file could not be read or is not usable.
    Save(SaveError),
    /// Content the save needs is missing from the installation.
    Blocked(Vec<Issue>),
}

impl RestoreError {
    /// A stable word for logs and the control protocol.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Save(e) => e.kind(),
            Self::Blocked(_) => "missing_content",
        }
    }
}

impl std::fmt::Display for RestoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Save(e) => e.fmt(f),
            Self::Blocked(issues) => {
                let list: Vec<&str> = issues
                    .iter()
                    .filter(|i| i.blocking)
                    .map(|i| i.subject.as_str())
                    .collect();
                write!(f, "missing content: {}", list.join(", "))
            }
        }
    }
}

impl From<SaveError> for RestoreError {
    fn from(e: SaveError) -> Self {
        Self::Save(e)
    }
}

/// A restore in progress.
#[derive(Resource, Debug, Clone)]
pub struct PendingRestore {
    /// The save's id.
    pub id: String,
    /// The save's name.
    pub name: String,
    /// Name of the first saved vehicle (the loading screen's vehicle line); empty without one.
    pub vehicle_label: String,
    pub(super) plan: RestorePlan,
    pub(super) world_applied: bool,
    pub(super) vehicles_applied: bool,
    pub(super) quiet_frames: u32,
    pub(super) waited_s: f32,
    /// Problems so far (sections that could not be applied, newer or unknown sections).
    pub warnings: Vec<String>,
    /// The main file was damaged and the backup was used.
    pub from_backup: bool,
}

/// How the last restore ended.
#[derive(Debug, Clone, PartialEq)]
pub enum RestoreOutcome {
    /// Everything that could be restored was.
    Done {
        /// The save.
        id: String,
        /// Problems on the way.
        warnings: Vec<String>,
    },
    /// The session could not be loaded (the loading screen shows why).
    Failed {
        /// The save.
        id: String,
    },
}

/// The outcome of the most recent restore.
#[derive(Resource, Debug, Clone, Default)]
pub struct LastRestore(pub Option<RestoreOutcome>);

/// What a started restore reports.
#[derive(Debug, Clone, PartialEq)]
pub struct RestoreStarted {
    /// The save's id.
    pub id: String,
    /// The save's name.
    pub name: String,
    /// Sections kept but not understood by this build.
    pub unknown_sections: Vec<String>,
    /// Sections the save lacks (they keep their defaults).
    pub missing_sections: Vec<String>,
    /// Content differences that did not block the load.
    pub warnings: Vec<String>,
}

pub(super) fn issue_text(i: &Issue) -> String {
    format!("{:?}: {}", i.kind, i.subject)
}
