//! What the governor remembers between frames: the window, the timers and the level change record.

use super::*;

/// What the governor remembers.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct GovernorState {
    /// The ladder level, `0..=5`.
    pub level: u8,
    /// The deadline budget (ms) of the latest frame (for reporting).
    pub target_ms: f32,
    /// The share of late frames in the window at the latest judgement (for reporting).
    pub late_share: f32,
    /// Seconds a level up currently needs (20, 40, 80 ... up to the cap).
    pub up_wait_s: f32,
    pub(super) window: VecDeque<f32>,
    pub(super) over_s: f32,
    pub(super) clean_s: f32,
    pub(super) since_step_s: f32,
    pub(super) since_up_s: f32,
    pub(super) since_down_s: f32,
    pub(super) warmup_s: f32,
    pub(super) hitch_run: u32,
}

impl Default for GovernorState {
    fn default() -> Self {
        Self {
            level: 0,
            target_ms: FALLBACK_VSYNC_MS,
            late_share: 0.0,
            up_wait_s: UP_AFTER_S,
            window: VecDeque::new(),
            over_s: 0.0,
            clean_s: 0.0,
            // The first step may happen as soon as the evidence is in.
            since_step_s: f32::MAX,
            since_up_s: f32::MAX,
            since_down_s: f32::MAX,
            warmup_s: 0.0,
            hitch_run: 0,
        }
    }
}

impl GovernorState {
    /// A governor that starts at `level` (clamped to `0..=5`).
    #[must_use]
    pub fn at(level: u8) -> Self {
        Self {
            level: level.min(MAX_LEVEL),
            ..Self::default()
        }
    }

    /// Back to level 0 with an empty window (custom to automatic).
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// The gate is closed (paused, loading, minimised, custom mode): forget the window and the
    /// timers and ignore the next [`GovernorSettings::warmup_s`] seconds once it opens again.
    /// The level and the up-backoff stay.
    pub fn pause(&mut self) {
        self.window.clear();
        self.over_s = 0.0;
        self.clean_s = 0.0;
        self.warmup_s = 0.0;
        self.hitch_run = 0;
    }

    /// Frames currently in the window.
    #[must_use]
    pub fn window_len(&self) -> usize {
        self.window.len()
    }

    pub(super) fn restart_window(&mut self) {
        self.window.clear();
        self.over_s = 0.0;
        self.clean_s = 0.0;
        self.since_step_s = 0.0;
    }
}

/// A level change the governor made.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Change {
    /// The level before.
    pub from: u8,
    /// The level now.
    pub to: u8,
    /// The deadline budget (ms) the frames were judged against.
    pub target_ms: f32,
    /// The share of late frames in the window that decided (0 for a level up).
    pub late_share: f32,
}
