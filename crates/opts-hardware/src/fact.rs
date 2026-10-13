//! A value that is either known or unknown *with a reason*.
//!
//! Reasons are a closed enum, never free text: nothing a probe read can leak through a reason.

use serde::{Deserialize, Serialize};

/// Why a fact is unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    /// The probe for this fact did not run (older file, or never asked).
    NotProbed,
    /// No probe exists for this fact on this platform.
    NotOnThisPlatform,
    /// The fact comes from the renderer or audio layer, which did not supply it.
    NeedsRenderer,
    /// The interface that holds the fact is absent (container, VM, old kernel).
    Unavailable,
    /// The interface exists but could not be read (permissions).
    Unreadable,
    /// The interface was read but its content was not understood.
    Unparsed,
    /// The 500 ms probe budget was used up before this stage ran.
    Timeout,
    /// An optional measurement that was not requested.
    NotRequested,
    /// A measurement was cancelled by the user.
    Cancelled,
}

impl Reason {
    /// Stable machine key (same as the serialised form).
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::NotProbed => "not_probed",
            Self::NotOnThisPlatform => "not_on_this_platform",
            Self::NeedsRenderer => "needs_renderer",
            Self::Unavailable => "unavailable",
            Self::Unreadable => "unreadable",
            Self::Unparsed => "unparsed",
            Self::Timeout => "timeout",
            Self::NotRequested => "not_requested",
            Self::Cancelled => "cancelled",
        }
    }

    /// Every reason (for the generated reference).
    pub const ALL: [Self; 9] = [
        Self::NotProbed,
        Self::NotOnThisPlatform,
        Self::NeedsRenderer,
        Self::Unavailable,
        Self::Unreadable,
        Self::Unparsed,
        Self::Timeout,
        Self::NotRequested,
        Self::Cancelled,
    ];

    /// One-line explanation (for the generated reference and the report).
    #[must_use]
    pub const fn describe(self) -> &'static str {
        match self {
            Self::NotProbed => "not probed (older profile or probe not run)",
            Self::NotOnThisPlatform => "no probe exists on this platform",
            Self::NeedsRenderer => {
                "supplied by the renderer or audio layer, which was not available"
            }
            Self::Unavailable => "the interface that holds it is absent",
            Self::Unreadable => "the interface could not be read",
            Self::Unparsed => "the content was not understood",
            Self::Timeout => "the probe budget was used up",
            Self::NotRequested => "an optional measurement that was not requested",
            Self::Cancelled => "cancelled by the user",
        }
    }
}

/// A known value or the reason it is unknown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Fact<T> {
    /// The value.
    Known(T),
    /// Unknown, with the reason.
    Unknown(Reason),
}

impl<T> Default for Fact<T> {
    fn default() -> Self {
        Self::Unknown(Reason::NotProbed)
    }
}

impl<T> Fact<T> {
    /// The value, when known.
    #[must_use]
    pub fn known(&self) -> Option<&T> {
        match self {
            Self::Known(v) => Some(v),
            Self::Unknown(_) => None,
        }
    }

    /// The reason, when unknown.
    #[must_use]
    pub const fn reason(&self) -> Option<Reason> {
        match self {
            Self::Known(_) => None,
            Self::Unknown(r) => Some(*r),
        }
    }

    /// Known value from an option, else `Unknown(reason)`.
    #[must_use]
    pub fn from_option(v: Option<T>, reason: Reason) -> Self {
        v.map_or(Self::Unknown(reason), Self::Known)
    }

    /// Maps a known value.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Fact<U> {
        match self {
            Self::Known(v) => Fact::Known(f(v)),
            Self::Unknown(r) => Fact::Unknown(r),
        }
    }
}

impl<T: Copy> Fact<T> {
    /// The value by copy.
    #[must_use]
    pub fn get(&self) -> Option<T> {
        self.known().copied()
    }
}

impl<T> From<Result<T, Reason>> for Fact<T> {
    fn from(r: Result<T, Reason>) -> Self {
        match r {
            Ok(v) => Self::Known(v),
            Err(e) => Self::Unknown(e),
        }
    }
}
