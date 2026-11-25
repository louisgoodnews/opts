//! Tuning of the governor: `GovernorSettings` with the named defaults.

use super::*;

/// Thresholds of the governor; overridable for tests. Defaults are the named constants.
///
/// The governor judges **missed deadlines**, not absolute frame time: see [`step`].
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct GovernorSettings {
    /// Frames in the sliding window.
    pub window_frames: usize,
    /// A frame is late above `late_factor * target_ms`.
    pub late_factor: f32,
    /// One level down when at least this share of a full window is late ...
    pub down_late_share: f32,
    /// ... for this long (s).
    pub down_after_s: f32,
    /// At most one step per this many seconds.
    pub min_step_interval_s: f32,
    /// One level up after this many seconds with no late frame in a full window (the first wait).
    pub up_after_s: f32,
    /// The wait doubles after every failed probation, up to this.
    pub up_after_max_s: f32,
    /// A level down within this long (s) of a level up is a failed probation.
    pub probation_s: f32,
    /// The wait is back to `up_after_s` after this long (s) without a level down.
    pub backoff_reset_s: f32,
    /// A frame longer than this (ms) is a hitch (tile streaming) and is not counted ...
    pub hitch_ms: f32,
    /// ... unless this many such frames follow each other: that is a real slowdown, not a hitch.
    pub hitch_run_limit: u32,
    /// Seconds ignored after the gate opens (shader/pipeline compilation).
    pub warmup_s: f32,
}

impl Default for GovernorSettings {
    fn default() -> Self {
        Self {
            window_frames: WINDOW_FRAMES,
            late_factor: LATE_FACTOR,
            down_late_share: DOWN_LATE_SHARE,
            down_after_s: DOWN_AFTER_S,
            min_step_interval_s: MIN_STEP_INTERVAL_S,
            up_after_s: UP_AFTER_S,
            up_after_max_s: UP_AFTER_MAX_S,
            probation_s: PROBATION_S,
            backoff_reset_s: BACKOFF_RESET_S,
            hitch_ms: HITCH_MS,
            hitch_run_limit: HITCH_RUN_LIMIT,
            warmup_s: WARMUP_S,
        }
    }
}
