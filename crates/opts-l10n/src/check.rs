//! Checking a translation against the English source: the rules behind `opts-tools l10n check`
//! and the language list of the game.
//!
//! Errors make a message unusable: it is **skipped** and the English text shows instead. Warnings
//! are reported and the message is kept.

use std::collections::BTreeSet;

use super::fluent::{self, Part, Pattern, Table};

mod stats;
pub use stats::{scripts_beyond_fonts, stats, Stats};

mod issue;
mod reference;

pub use issue::{Issue, Severity};
pub use reference::Reference;

/// A table after validation: only the messages that passed, and the findings.
#[derive(Debug, Clone, Default)]
pub struct Checked {
    /// The accepted messages (`messages`, `lines`, `order`, `comments` are filled).
    pub table: Table,
    /// All findings.
    pub issues: Vec<Issue>,
}

mod rules;
pub use rules::*;

#[cfg(test)]
mod tests;
