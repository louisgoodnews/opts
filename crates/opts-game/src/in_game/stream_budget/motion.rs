//! The player's speed and heading, estimated from where the vehicle is each frame (M-0114).

/// A move farther than this between two frames is a teleport: the speed is reset, not measured.
/// The same 12 m rule that the vehicle uses to snap after a teleport.
pub const TELEPORT_M: f32 = 12.0;
/// Speed smoothing rate (per second) of the measured speed.
const SMOOTHING_PER_S: f32 = 4.0;
/// Below this displacement (m) the heading is kept.
const MIN_STEP_M: f32 = 1e-3;

/// Speed and heading of the player in the scene's ground plane (x east, y north).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Motion {
    last: Option<(f32, f32)>,
    speed_mps: f32,
    heading: (f32, f32),
}

impl Motion {
    /// Feeds the position of this frame and the time since the last one. Returns `true` when the
    /// position jumped (a teleport), in which case the speed is zero.
    pub fn observe(&mut self, pos: (f32, f32), dt: f32) -> bool {
        let mut teleported = false;
        if let Some(prev) = self.last {
            let step = (pos.0 - prev.0, pos.1 - prev.1);
            let dist = step.0.hypot(step.1);
            if dist > TELEPORT_M {
                self.speed_mps = 0.0;
                teleported = true;
            } else if dt > 0.0 && dist.is_finite() {
                let instant = dist / dt;
                let alpha = 1.0 - (-SMOOTHING_PER_S * dt).exp();
                self.speed_mps += (instant - self.speed_mps) * alpha;
            }
            if dist > MIN_STEP_M && dist <= TELEPORT_M {
                self.heading = (step.0 / dist, step.1 / dist);
            }
        }
        self.last = Some(pos);
        teleported
    }

    /// The measured speed in m/s.
    #[must_use]
    pub fn speed_mps(&self) -> f32 {
        self.speed_mps
    }

    /// The direction of travel (unit vector; the last direction while standing still). The
    /// displacement is used, so reversing counts as travelling backwards.
    #[must_use]
    pub fn heading(&self) -> (f32, f32) {
        self.heading
    }
}
