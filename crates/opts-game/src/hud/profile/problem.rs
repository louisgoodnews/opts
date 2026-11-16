//! Problems found while reading a HUD profile file.

use super::*;

/// A problem found while reading a profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileProblem {
    /// The text is not valid RON for a profile.
    Syntax(String),
    /// An id that no widget has.
    UnknownWidget(String),
    /// A slot the widget does not allow.
    SlotNotAllowed(String, Slot),
    /// The same widget twice.
    Duplicate(String),
}

impl std::fmt::Display for ProfileProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Syntax(e) => write!(f, "not a HUD profile: {e}"),
            Self::UnknownWidget(id) => write!(f, "unknown widget `{id}`"),
            Self::SlotNotAllowed(id, s) => write!(f, "widget `{id}` cannot be in slot {}", s.id()),
            Self::Duplicate(id) => write!(f, "widget `{id}` listed twice"),
        }
    }
}
