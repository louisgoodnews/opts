//! Frame times of the running drive: counted after the warm-up, averaged and the 95th percentile.

use bevy::prelude::*;
use crate::in_game::types::SimulationLoadState;

/// Seconds after the load finished that are not counted (shader and tile warm-up).
pub const WARMUP_S: f32 = 10.0;
const BINS: usize = 400;

/// The frame times of the running drive.
#[derive(Resource, Debug, Clone)]
pub struct FrameStats {
    since_loaded_s: f32,
    seconds: f32,
    sum_ms: f64,
    frames: u64,
    bins: Vec<u32>,
}

impl Default for FrameStats {
    fn default() -> Self {
        Self {
            since_loaded_s: 0.0,
            seconds: 0.0,
            sum_ms: 0.0,
            frames: 0,
            bins: vec![0; BINS],
        }
    }
}

impl FrameStats {
    /// Counts one frame of `dt_s` seconds once the warm-up of [`WARMUP_S`] is over.
    pub fn frame(&mut self, dt_s: f32) {
        self.since_loaded_s += dt_s;
        if self.since_loaded_s <= WARMUP_S || !dt_s.is_finite() || dt_s <= 0.0 {
            return;
        }
        let ms = f64::from(dt_s) * 1000.0;
        self.seconds += dt_s;
        self.sum_ms += ms;
        self.frames += 1;
        self.bins[(ms as usize).min(BINS - 1)] += 1;
    }

    /// Average frame time (ms), none before a frame was counted.
    #[must_use]
    pub fn average_ms(&self) -> Option<f32> {
        (self.frames > 0).then(|| (self.sum_ms / self.frames as f64) as f32)
    }

    /// 95th-percentile frame time (ms; 1 ms resolution).
    #[must_use]
    pub fn p95_ms(&self) -> Option<f32> {
        if self.frames == 0 {
            return None;
        }
        let target = (self.frames as f64 * 0.95).ceil() as u64;
        let mut seen = 0u64;
        for (i, n) in self.bins.iter().enumerate() {
            seen += u64::from(*n);
            if seen >= target {
                return Some(i as f32 + 1.0);
            }
        }
        Some(BINS as f32)
    }

    /// Seconds counted (after the warm-up).
    #[must_use]
    pub fn seconds(&self) -> f32 {
        self.seconds
    }

    pub(super) fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Counts the frame time of the drive.
pub fn track_frame_time(
    time: Res<Time<Real>>,
    virtual_time: Res<Time<Virtual>>,
    load: Option<Res<SimulationLoadState>>,
    mut stats: ResMut<FrameStats>,
) {
    if !load.is_some_and(|l| l.is_complete) || virtual_time.is_paused() {
        return;
    }
    stats.frame(time.delta_secs());
}
