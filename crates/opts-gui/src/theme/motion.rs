//! Material Design 3 motion tokens, durations, and easing curves.

use std::time::Duration;

/// Standard animation and transition duration tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DurationToken {
    Short,
    Medium,
    Long,
}

impl DurationToken {
    /// Duration in milliseconds.
    pub const fn millis(&self) -> u64 {
        match self {
            Self::Short => 100,
            Self::Medium => 250,
            Self::Long => 400,
        }
    }

    /// Converts to standard Rust [`Duration`].
    pub const fn as_duration(&self) -> Duration {
        Duration::from_millis(self.millis())
    }
}

/// Standard easing curves.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EasingCurve {
    Standard,
    Emphasized,
    Decelerated,
    Accelerated,
}

/// Motion configuration tokens.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MotionTokens {
    pub reduced_motion: bool,
}

impl MotionTokens {
    /// Retrieves the duration in milliseconds, respecting reduced motion mode.
    pub fn duration(&self, token: DurationToken) -> Duration {
        if self.reduced_motion {
            Duration::ZERO
        } else {
            token.as_duration()
        }
    }
}
