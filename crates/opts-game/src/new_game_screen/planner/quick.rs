//! The quick time-of-day chips of step When and the time arithmetic behind them.

use super::super::types::NewGameSetup;

/// A quick time of day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuickTime {
    /// Message key of the chip label.
    pub key: &'static str,
    /// Hour, or `None` for *Now* (the wall clock).
    pub hour: Option<u32>,
    /// Minute (ignored for *Now*).
    pub minute: u32,
}

/// The quick chips of step *When*, in order.
pub const QUICK_TIMES: [QuickTime; 5] = [
    QuickTime {
        key: "wiz-when-now",
        hour: None,
        minute: 0,
    },
    QuickTime {
        key: "wiz-when-morning",
        hour: Some(8),
        minute: 0,
    },
    QuickTime {
        key: "wiz-when-noon",
        hour: Some(12),
        minute: 0,
    },
    QuickTime {
        key: "wiz-when-evening",
        hour: Some(18),
        minute: 0,
    },
    QuickTime {
        key: "wiz-when-night",
        hour: Some(22),
        minute: 0,
    },
];

/// Rounds a time of day to 5 minutes (a result of 24:00 wraps to 00:00).
pub fn round_to_five(hour: u32, minute: u32) -> (u32, u32) {
    let total = ((hour * 60 + minute + 2) / 5) * 5;
    ((total / 60) % 24, total % 60)
}

/// The time of day a chip sets, given the wall clock for *Now*.
pub fn quick_time_of(chip: &QuickTime, wall: (u32, u32)) -> (u32, u32) {
    match chip.hour {
        Some(h) => (h, chip.minute),
        None => round_to_five(wall.0, wall.1),
    }
}

/// The index of the chip that matches the current time exactly (never *Now*, which moves).
pub fn matching_quick_time(setup: &NewGameSetup) -> Option<usize> {
    QUICK_TIMES
        .iter()
        .position(|q| q.hour == Some(setup.hour) && q.minute == setup.minute)
}

/// `HH:MM`.
pub fn clock_text(setup: &NewGameSetup) -> String {
    format!("{:02}:{:02}", setup.hour, setup.minute)
}
