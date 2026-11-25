//! The head pose a tracking device reports, and how the driver's view takes it.

/// Largest yaw the view follows, in degrees to either side of the centre.
// TODO(owner): ISS-045 gives no limit; 90 degrees is a placeholder.
pub const HEAD_YAW_LIMIT_DEG: f32 = 90.0;
/// Largest pitch the view follows, in degrees up or down from the centre.
// TODO(owner): ISS-045 gives no limit; 45 degrees is a placeholder.
pub const HEAD_PITCH_LIMIT_DEG: f32 = 45.0;

/// A head pose. Angles are radians, yaw positive to the right and pitch positive up. The
/// translation is metres in the driver's frame (X right, Y up, Z forward). The camera uses yaw
/// and pitch; roll and translation are carried for later use.
// TODO(owner): decide whether roll and the head translation drive the eye point (M-0166).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct HeadPose {
    /// Turn to the right (positive) or left, radians.
    pub yaw: f32,
    /// Tilt up (positive) or down, radians.
    pub pitch: f32,
    /// Roll to the right (positive), radians.
    pub roll: f32,
    /// Head position to the right, metres.
    pub x: f32,
    /// Head position up, metres.
    pub y: f32,
    /// Head position forward, metres.
    pub z: f32,
}

impl HeadPose {
    /// Whether every value is finite (a device may report garbage).
    #[must_use]
    pub fn is_finite(&self) -> bool {
        [self.yaw, self.pitch, self.roll, self.x, self.y, self.z]
            .iter()
            .all(|v| v.is_finite())
    }

    /// The look offset `(yaw, pitch)` in radians that the camera adds for this pose, relative to
    /// `centre` and scaled by `gain`, each limited. The view's yaw is positive to the left, so the
    /// head's yaw is negated.
    #[must_use]
    pub fn look_offset(&self, centre: &Self, gain: f32) -> (f32, f32) {
        let yaw_limit = HEAD_YAW_LIMIT_DEG.to_radians();
        let pitch_limit = HEAD_PITCH_LIMIT_DEG.to_radians();
        let yaw = -(self.yaw - centre.yaw) * gain;
        let pitch = (self.pitch - centre.pitch) * gain;
        (
            yaw.clamp(-yaw_limit, yaw_limit),
            pitch.clamp(-pitch_limit, pitch_limit),
        )
    }
}
