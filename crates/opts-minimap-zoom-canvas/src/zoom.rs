//! Speed-based zoom of the mini map: slow or stopped shows a small area, fast shows a large one.
//!
//! Pure functions and one small filter, no Bevy types, so the behaviour is tested without an app.
//! The design and the constants are explained in `docs/src/architecture/minimap.md`.

/// Speed (km/h) at and below which the map shows its near radius.
pub const V_SLOW_KMH: f32 = 5.0;
/// Speed (km/h) at and above which the map shows the user's radius.
pub const V_FAST_KMH: f32 = 60.0;
/// The near radius as a share of the user's radius.
pub const NEAR_RATIO: f32 = 1.0 / 3.0;
/// The near radius never goes below this (metres).
pub const NEAR_MIN_M: f32 = 40.0;
/// Time constant of the speed filter (seconds).
pub const TAU_SPEED_S: f32 = 1.5;
/// Time constant while the radius grows (zooming out), seconds.
pub const TAU_OUT_S: f32 = 0.7;
/// Time constant while the radius shrinks (zooming in), seconds.
pub const TAU_IN_S: f32 = 2.0;
/// The radius does not move while `|ln target - ln radius|` is below this.
pub const DEAD_BAND: f32 = 0.02;
/// A jump of the map centre larger than this within one tick restarts the filter (metres).
pub const JUMP_M: f64 = 100.0;

/// The near radius for the user's radius `far_m`.
#[must_use]
pub fn near_radius(far_m: f32) -> f32 {
    (far_m * NEAR_RATIO).max(NEAR_MIN_M).min(far_m)
}

/// The radius the map wants at `speed_kmh`: the near radius up to [`V_SLOW_KMH`], `far_m` from
/// [`V_FAST_KMH`] on, and a smooth interpolation in logarithmic space between.
#[must_use]
pub fn target_radius(speed_kmh: f32, far_m: f32) -> f32 {
    let near = near_radius(far_m);
    let t = ((speed_kmh - V_SLOW_KMH) / (V_FAST_KMH - V_SLOW_KMH)).clamp(0.0, 1.0);
    let s = t * t * (3.0 - 2.0 * t);
    (near.ln() + s * (far_m.ln() - near.ln())).exp()
}

/// The smoothing state: the filtered speed and the radius (as a logarithm).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ZoomFilter {
    /// Filtered speed, km/h.
    pub speed_kmh: f32,
    /// Natural logarithm of the current radius in metres.
    ln_radius: f32,
    initialised: bool,
}

impl ZoomFilter {
    /// Forgets everything; the next [`step`](Self::step) starts at its target.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// The current radius in metres, once the filter has run.
    #[must_use]
    pub fn radius_m(&self) -> Option<f32> {
        self.initialised.then(|| self.ln_radius.exp())
    }

    /// Advances by `dt` seconds with the measured speed and the user's radius; returns the radius
    /// to draw. The first call (and the first after [`reset`](Self::reset)) returns the target
    /// radius of the measured speed without animation.
    pub fn step(&mut self, dt: f32, speed_in_kmh: f32, far_m: f32) -> f32 {
        let dt = dt.clamp(0.0, 1.0);
        let speed_in = speed_in_kmh.max(0.0);
        if !self.initialised {
            self.speed_kmh = speed_in;
            self.ln_radius = target_radius(speed_in, far_m).ln();
            self.initialised = true;
            return self.ln_radius.exp();
        }
        self.speed_kmh += (speed_in - self.speed_kmh) * (1.0 - (-dt / TAU_SPEED_S).exp());
        let want = target_radius(self.speed_kmh, far_m).ln();
        let error = want - self.ln_radius;
        if error.abs() >= DEAD_BAND {
            let tau = if error > 0.0 { TAU_OUT_S } else { TAU_IN_S };
            self.ln_radius += error * (1.0 - (-dt / tau).exp());
        }
        self.ln_radius.exp()
    }
}

/// Speed (km/h) from two consecutive centres of the map, for a session without a vehicle.
/// `None` when the jump is a teleport (more than [`JUMP_M`]) or `dt` is not positive.
#[must_use]
pub fn camera_speed_kmh(moved_m: f64, dt: f32) -> Option<f32> {
    if dt <= 0.0 || moved_m > JUMP_M {
        return None;
    }
    Some((moved_m / f64::from(dt) * 3.6) as f32)
}
