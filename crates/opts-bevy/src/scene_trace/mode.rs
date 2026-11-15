//! The trace mode: how much is kept (`off`, `summary`, `full`) and its parsing.

use serde::{Deserialize, Serialize};

/// How much the trace keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Nothing is kept (default).
    #[default]
    Off,
    /// Counts per tile and kind, and a record for every thing with an issue.
    Summary,
    /// A record for every placed thing.
    Full,
}

impl Mode {
    /// Parses `off`, `summary` or `full` (case-insensitive); the empty text is `off`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_lowercase().as_str() {
            "" | "off" | "false" | "0" | "no" => Some(Self::Off),
            "summary" | "sum" | "true" | "1" | "on" => Some(Self::Summary),
            "full" | "all" | "2" => Some(Self::Full),
            _ => None,
        }
    }

    /// The text form (`off`, `summary`, `full`).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Summary => "summary",
            Self::Full => "full",
        }
    }

    pub(super) const fn code(self) -> u8 {
        match self {
            Self::Off => 0,
            Self::Summary => 1,
            Self::Full => 2,
        }
    }

    pub(super) const fn from_code(c: u8) -> Self {
        match c {
            1 => Self::Summary,
            2 => Self::Full,
            _ => Self::Off,
        }
    }
}
