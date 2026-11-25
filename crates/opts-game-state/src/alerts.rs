//! The alert types that sources write and the queue reads: the severity and the raise message.
//! The queue, its systems and the slot are in `opts-game` (`hud::alerts`), which re-exports these.

use bevy::prelude::*;
use opts_hud_keys as keys;

/// How serious an alert is; decides the colour, the word, and whether it stays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertSeverity {
    /// Something worth knowing; leaves by itself.
    Info,
    /// Something to act on soon; leaves by itself.
    Warning,
    /// Something to act on now; stays until its source clears it.
    Critical,
}

impl AlertSeverity {
    /// Message key of the word shown before the text (colour is never the only signal).
    #[must_use]
    pub const fn word_key(self) -> &'static str {
        match self {
            Self::Info => keys::ALERT_INFO,
            Self::Warning => keys::ALERT_WARNING,
            Self::Critical => keys::ALERT_CRITICAL,
        }
    }

    /// Whether the alert stays until cleared.
    #[must_use]
    pub const fn sticky(self) -> bool {
        matches!(self, Self::Critical)
    }

    /// Stable id (control protocol).
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Critical => "critical",
        }
    }

    /// Parses an [`id`](Self::id).
    #[must_use]
    pub fn parse(id: &str) -> Option<Self> {
        [Self::Info, Self::Warning, Self::Critical]
            .into_iter()
            .find(|s| s.id() == id)
    }
}

/// Asks for an alert. The same `id` again replaces the earlier one (new severity, new timer).
#[derive(Message, Debug, Clone)]
pub struct RaiseAlert {
    /// Identifies the source (`brake.moving`).
    pub id: &'static str,
    /// Severity.
    pub severity: AlertSeverity,
    /// Message key of the text.
    pub key: &'static str,
    /// Variables of the text.
    pub args: Vec<(&'static str, String)>,
}
