//! Height, pitch and roll of the units of a consist on the ground, computed in the fixed step
//! (design D3, `docs/src/architecture/articulation-physics.md`).
//!
//! `opts-sim` has no ground: the host samples four heights per unit (rear axle and front joint,
//! each at both sides) at the points [`GroundProbe::points`] names and hands them to
//! [`VehicleSimulation::apply_ground`](super::VehicleSimulation::apply_ground). Everything
//! else — the goal pose, the first-order filter with the fixed time step, the joint variables
//! `β` and `γ` — is computed here, so it is a function of the fixed step alone.
//!
//! Conventions: pitch is nose up positive, roll right side up positive, `β = leader pitch -
//! pitch` and `γ = roll - leader roll` (`docs/src/architecture/bellows-census.md`).

use glam::Vec2;
use serde::{Deserialize, Serialize};

/// Largest pitch and roll of a body (rad).
pub const MAX_TILT_RAD: f32 = 0.35;
/// Response rate of the height onto the ground (1/s).
pub const RISE_RATE: f32 = 25.0;
/// Response rate of pitch and roll (1/s).
pub const TILT_RATE: f32 = 10.0;

/// Where a unit stands and which ground points describe it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroundProbe {
    /// World position of the unit's origin (simulation space).
    pub origin: Vec2,
    /// Heading of the unit (compass radians).
    pub heading: f32,
    /// Position `y` of the rear axle in the unit's frame.
    pub rear_y: f32,
    /// Position `y` of the front axle (head) or front joint (following unit).
    pub front_y: f32,
    /// Track width the sides are sampled at (m).
    pub track_m: f32,
}

impl GroundProbe {
    /// The four world points to sample, in the order of [`GroundSample::from_heights`]:
    /// rear left, rear right, front left, front right (left is `-x` of the unit's frame).
    #[must_use]
    pub fn points(&self) -> [Vec2; 4] {
        let (sin, cos) = self.heading.sin_cos();
        let at = |ly: f32, side: f32| {
            let lat = side * self.track_m * 0.5;
            Vec2::new(
                self.origin.x + sin * ly + cos * lat,
                self.origin.y + cos * ly - sin * lat,
            )
        };
        [
            at(self.rear_y, -1.0),
            at(self.rear_y, 1.0),
            at(self.front_y, -1.0),
            at(self.front_y, 1.0),
        ]
    }

    fn length(&self) -> f32 {
        (self.front_y - self.rear_y).max(1.0)
    }
}

/// Ground heights (m) at the four points of a [`GroundProbe`], with its geometry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroundSample {
    /// Rear left, rear right, front left, front right.
    pub heights: [f32; 4],
    /// The probe the heights were taken at.
    pub probe: GroundProbe,
}

impl GroundSample {
    /// Pairs the heights (in the order of [`GroundProbe::points`]) with their probe.
    #[must_use]
    pub fn from_heights(probe: GroundProbe, heights: [f32; 4]) -> Self {
        Self { heights, probe }
    }

    /// The pose the unit settles to on this ground: height at its origin, pitch and roll.
    #[must_use]
    pub fn goal(&self) -> UnitBody {
        let [rl, rr, fl, fr] = self.heights;
        let length = self.probe.length();
        let (g_rear, g_front) = ((rl + rr) * 0.5, (fl + fr) * 0.5);
        // The origin lies between the rear axle and the front joint (or beyond them).
        let t = ((0.0 - self.probe.rear_y) / length).clamp(-1.0, 2.0);
        UnitBody {
            elevation: g_rear + (g_front - g_rear) * t,
            pitch: ((g_front - g_rear) / length)
                .atan()
                .clamp(-MAX_TILT_RAD, MAX_TILT_RAD),
            roll: (((fr + rr) - (fl + rl)) * 0.5 / self.probe.track_m.max(0.5))
                .atan()
                .clamp(-MAX_TILT_RAD, MAX_TILT_RAD),
        }
    }
}

/// Height, pitch and roll of a unit (filtered state).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct UnitBody {
    /// Height of the unit's origin above the datum (m).
    pub elevation: f32,
    /// Pitch, nose up positive (rad).
    pub pitch: f32,
    /// Roll, right side up positive (rad).
    pub roll: f32,
}

impl UnitBody {
    /// Moves the state towards `goal` by one step of `dt` seconds (first-order filters, rates
    /// [`RISE_RATE`] and [`TILT_RATE`]).
    pub fn settle(&mut self, goal: UnitBody, dt: f32) {
        let rise = 1.0 - (-RISE_RATE * dt).exp();
        let tilt = 1.0 - (-TILT_RATE * dt).exp();
        self.elevation += (goal.elevation - self.elevation) * rise;
        self.pitch += (goal.pitch - self.pitch) * tilt;
        self.roll += (goal.roll - self.roll) * tilt;
    }
}

/// `β` in degrees for a unit pitched `pitch` behind one pitched `leader_pitch` (nose up
/// positive): the end of the bellows joined to a nose-up follower turns the other way.
#[must_use]
pub fn joint_beta_deg(leader_pitch: f32, pitch: f32) -> f32 {
    (leader_pitch - pitch).to_degrees()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn probe(heading: f32) -> GroundProbe {
        GroundProbe {
            origin: Vec2::new(10.0, 20.0),
            heading,
            rear_y: -6.0,
            front_y: 1.0,
            track_m: 2.0,
        }
    }

    #[test]
    fn flat_ground_gives_zero_pitch_and_roll() {
        let g = GroundSample::from_heights(probe(0.7), [3.0; 4]).goal();
        assert!(g.pitch.abs() < 1e-6 && g.roll.abs() < 1e-6);
        assert!((g.elevation - 3.0).abs() < 1e-6);
    }

    #[test]
    fn a_constant_slope_gives_the_slope_angle_and_the_joint_difference() {
        // 10 % grade along the heading: pitch is atan(0.1) for every unit, so β is 0; a unit
        // on a different grade gives β = difference of the two angles.
        let p = probe(0.0);
        let [r, f] = [-6.0_f32, 1.0];
        let up = |slope: f32| {
            GroundSample::from_heights(p, [r * slope, r * slope, f * slope, f * slope]).goal()
        };
        let a = up(0.1);
        assert!((a.pitch - 0.1_f32.atan()).abs() < 1e-5);
        let b = up(0.0);
        let beta = joint_beta_deg(a.pitch, b.pitch);
        assert!((beta - 0.1_f32.atan().to_degrees()).abs() < 1e-3);
        assert!(joint_beta_deg(a.pitch, a.pitch).abs() < 1e-6);
    }

    #[test]
    fn a_cross_slope_gives_positive_roll_when_the_right_side_is_up() {
        let g = GroundSample::from_heights(probe(0.0), [0.0, 0.2, 0.0, 0.2]).goal();
        assert!((g.roll - (0.1_f32).atan()).abs() < 1e-5, "{}", g.roll);
        // Facing east, "right" is south: the same heights read at a rotated probe still map
        // to the unit's own sides.
        let pts = probe(std::f32::consts::FRAC_PI_2).points();
        assert!((pts[1].y - pts[0].y + 2.0).abs() < 1e-4, "{pts:?}");
    }

    #[test]
    fn the_filter_depends_on_the_step_only_through_dt() {
        let goal = UnitBody {
            elevation: 1.0,
            pitch: 0.2,
            roll: -0.1,
        };
        let mut a = UnitBody::default();
        let mut b = UnitBody::default();
        for _ in 0..64 {
            a.settle(goal, 1.0 / 64.0);
        }
        for _ in 0..128 {
            b.settle(goal, 1.0 / 128.0);
        }
        // Same time, different step: the first-order filter is exact for a constant goal.
        assert!((a.pitch - b.pitch).abs() < 1e-5 && (a.elevation - b.elevation).abs() < 1e-5);
        assert!(a.pitch > 0.19);
    }
}
