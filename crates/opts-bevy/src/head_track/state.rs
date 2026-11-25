//! The head-tracking resources: the settings copied from the game, and the device state.

use bevy::prelude::*;

use super::device::HeadPoseSource;
use super::pose::HeadPose;

/// The gain when the setting is missing: one degree of head turn is one degree of view.
// TODO(owner): ISS-045 gives no gain; 1.0 is a placeholder.
pub const DEFAULT_GAIN: f32 = 1.0;
/// The largest gain the setting allows.
// TODO(owner): ISS-045 gives no range; 2.0 is a placeholder.
pub const MAX_GAIN: f32 = 2.0;

/// The settings of head tracking, copied from the game's settings each frame.
#[derive(Debug, Clone, Copy, PartialEq, Resource)]
pub struct HeadTrackSettings {
    /// Whether the driver's view follows the head.
    pub enabled: bool,
    /// The look per degree of head turn.
    pub gain: f32,
}

impl Default for HeadTrackSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            gain: DEFAULT_GAIN,
        }
    }
}

/// The open device, its newest pose and the centre.
#[derive(Resource, Default)]
pub struct HeadTrackState {
    /// The open device, `None` while head tracking is off.
    pub source: Option<Box<dyn HeadPoseSource>>,
    /// The newest pose the device reported.
    pub latest: HeadPose,
    /// The pose that counts as straight ahead.
    pub centre: HeadPose,
    /// Set by a centre request; the next frame makes [`Self::latest`] the centre.
    pub centre_requested: bool,
    /// Whether the device has reported a pose since it was opened.
    pub has_pose: bool,
    /// Whether the current run of device errors has been logged.
    pub error_reported: bool,
}

impl HeadTrackState {
    /// Asks for the current pose to become the centre (applied in the next frame).
    pub fn request_centre(&mut self) {
        self.centre_requested = true;
    }

    /// The look offset `(yaw, pitch)` in radians for the driver's view: zero while head tracking
    /// is off or no device is open.
    #[must_use]
    pub fn look(&self, settings: &HeadTrackSettings) -> (f32, f32) {
        if !settings.enabled || self.source.is_none() {
            return (0.0, 0.0);
        }
        self.latest.look_offset(&self.centre, settings.gain)
    }
}
