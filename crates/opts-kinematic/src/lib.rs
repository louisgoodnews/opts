//! The cheap kinematic acceleration model of an AI vehicle (M-0103): its acceleration from the
//! drive torque measured over speed, without running the vehicle's script each frame.
//!
//! A [`DriveCurve`] holds the torque at full throttle at sampled speeds, linear between the
//! samples and held beyond them. Acceleration is `torque / (wheel radius · mass)`, the law the
//! traffic model uses for its peak torque.

/// Torque (N·m) at full throttle over speed (km/h), piecewise linear.
#[derive(Debug, Clone, PartialEq)]
pub struct DriveCurve {
    points: Vec<(f32, f32)>,
}

impl DriveCurve {
    /// A curve through `points` (speed km/h, torque N·m), sorted by speed.
    #[must_use]
    pub fn new(mut points: Vec<(f32, f32)>) -> Self {
        points.sort_by(|a, b| a.0.total_cmp(&b.0));
        Self { points }
    }

    /// Torque at `kmh`: linear between the samples, clamped at both ends (`0` without samples).
    #[must_use]
    pub fn torque_at(&self, kmh: f32) -> f32 {
        let Some(&(first_kmh, first)) = self.points.first() else {
            return 0.0;
        };
        if kmh <= first_kmh {
            return first;
        }
        for w in self.points.windows(2) {
            let ((ka, ta), (kb, tb)) = (w[0], w[1]);
            if kmh <= kb {
                return ta + (tb - ta) * (kmh - ka) / (kb - ka).max(1e-6);
            }
        }
        self.points.last().map_or(0.0, |p| p.1)
    }

    /// Acceleration (m/s²) at speed `v_ms` for a vehicle of `mass_kg` on wheels of `radius_m`.
    #[must_use]
    pub fn acceleration_at(&self, v_ms: f32, mass_kg: f32, radius_m: f32) -> f32 {
        self.torque_at(v_ms * 3.6) / (radius_m.max(0.01) * mass_kg.max(1.0))
    }

    /// Time (s) and distance (m) to reach `target_ms` from rest, stepping `dt` s; `None` when
    /// the vehicle does not reach the target within `limit_s` s.
    #[must_use]
    pub fn run_up(
        &self,
        target_ms: f32,
        mass_kg: f32,
        radius_m: f32,
        dt: f32,
        limit_s: f32,
    ) -> Option<(f32, f32)> {
        run_up_with(
            |kmh, _| self.torque_at(kmh),
            target_ms,
            mass_kg,
            radius_m,
            dt,
            limit_s,
        )
    }
}

/// Run-up from rest with the drive torque `torque(km/h, dt)` at each step: the same law as
/// [`DriveCurve::run_up`], for a torque that a script gives frame by frame.
#[must_use]
pub fn run_up_with(
    mut torque: impl FnMut(f32, f32) -> f32,
    target_ms: f32,
    mass_kg: f32,
    radius_m: f32,
    dt: f32,
    limit_s: f32,
) -> Option<(f32, f32)> {
    let (mut v, mut x, mut t) = (0.0_f32, 0.0_f32, 0.0_f32);
    while t < limit_s {
        let a = torque(v * 3.6, dt) / (radius_m.max(0.01) * mass_kg.max(1.0));
        v = (v + a * dt).max(0.0);
        x += v * dt;
        t += dt;
        if v >= target_ms {
            return Some((t, x));
        }
    }
    None
}
