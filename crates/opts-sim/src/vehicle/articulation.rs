//! Articulation joint kinematics for multi-unit transit vehicles (articulated buses).
//!
//! Provides geometric hitch constraint tracking, turntable relative angle
//! calculation, mechanical rotation limits, and kinematic trailer following.

use glam::Vec2;
use serde::{Deserialize, Serialize};

/// Physical state of an articulation turntable / coupling joint.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ArticulationState {
    /// Relative articulation angle between tractor and trailer headings in radians.
    pub angle_rad: f32,
    /// Rate of articulation angle change in radians per second.
    pub angular_velocity_rad_s: f32,
    /// Absolute 2D world position of the coupling pivot pin.
    pub hitch_world_pos: Vec2,
    /// Heading angle of the trailing unit in world space in radians.
    pub trailer_heading_rad: f32,
}

/// Kinematic articulation joint connecting leading and trailing vehicle sections.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArticulationJoint {
    /// Longitudinal position of the coupling point on the leading tractor unit in meters (+Y = forward).
    pub tractor_hitch_y: f32,
    /// Longitudinal position of the coupling point on the trailing trailer unit in meters.
    pub trailer_hitch_y: f32,
    /// Distance from the coupling pivot to the trailer's effective rear axle in meters.
    pub trailer_wheelbase_m: f32,
    /// Maximum mechanical turntable rotation angle limit in radians (typically ~54 degrees).
    pub max_angle_rad: f32,
    /// Current relative articulation turntable angle in radians (+ = articulated right).
    pub current_angle_rad: f32,
    /// Current trailer heading in world coordinates in radians.
    pub trailer_heading_rad: f32,
}

impl Default for ArticulationJoint {
    fn default() -> Self {
        Self {
            tractor_hitch_y: -5.5,
            trailer_hitch_y: 1.2,
            trailer_wheelbase_m: 6.2,
            max_angle_rad: 0.942, // ~54 degrees
            current_angle_rad: 0.0,
            trailer_heading_rad: 0.0,
        }
    }
}

impl ArticulationJoint {
    /// Creates a new articulation joint with specified geometric mounting positions.
    #[must_use]
    pub fn new(tractor_hitch_y: f32, trailer_hitch_y: f32, trailer_wheelbase_m: f32) -> Self {
        Self {
            tractor_hitch_y,
            trailer_hitch_y,
            trailer_wheelbase_m: trailer_wheelbase_m.max(1.0),
            max_angle_rad: 0.942,
            current_angle_rad: 0.0,
            trailer_heading_rad: 0.0,
        }
    }

    /// Computes world position of the hitch pin based on tractor world position and heading.
    #[must_use]
    pub fn compute_hitch_world_pos(&self, tractor_pos: Vec2, tractor_heading_rad: f32) -> Vec2 {
        let forward = Vec2::new(tractor_heading_rad.sin(), tractor_heading_rad.cos());
        tractor_pos + forward * self.tractor_hitch_y
    }

    /// Updates articulation kinematics for one simulation time step.
    ///
    /// # Arguments
    /// * `tractor_pos` - World position of tractor reference point.
    /// * `tractor_heading_rad` - World yaw heading angle of tractor in radians.
    /// * `tractor_speed_m_s` - Forward speed of tractor in meters per second.
    /// * `dt` - Simulation step duration in seconds.
    ///
    /// # Returns
    /// An updated [`ArticulationState`].
    pub fn step(
        &mut self,
        tractor_pos: Vec2,
        tractor_heading_rad: f32,
        tractor_speed_m_s: f32,
        dt: f32,
    ) -> ArticulationState {
        let hitch_pos = self.compute_hitch_world_pos(tractor_pos, tractor_heading_rad);

        // Kinematic following equation: trailer yaw rate d_psi / dt = (v * sin(psi_art)) / L
        let yaw_rate =
            (tractor_speed_m_s * self.current_angle_rad.sin()) / self.trailer_wheelbase_m;

        self.trailer_heading_rad += yaw_rate * dt;

        // Relative angle: tractor heading - trailer heading
        let raw_angle = tractor_heading_rad - self.trailer_heading_rad;
        let clamped_angle = raw_angle.clamp(-self.max_angle_rad, self.max_angle_rad);

        let delta_angle = clamped_angle - self.current_angle_rad;
        let angular_velocity = if dt > 0.0001 { delta_angle / dt } else { 0.0 };

        self.current_angle_rad = clamped_angle;

        ArticulationState {
            angle_rad: self.current_angle_rad,
            angular_velocity_rad_s: angular_velocity,
            hitch_world_pos: hitch_pos,
            trailer_heading_rad: self.trailer_heading_rad,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_straight_line_articulation_zero() {
        let mut joint = ArticulationJoint::default();
        let tractor_pos = Vec2::new(0.0, 0.0);
        let heading = 0.0; // Facing along Y axis

        for _ in 0..10 {
            let state = joint.step(tractor_pos, heading, 10.0, 0.016);
            assert_eq!(state.angle_rad, 0.0);
            assert_eq!(state.angular_velocity_rad_s, 0.0);
        }
    }

    #[test]
    fn test_articulation_angle_limiting() {
        let mut joint = ArticulationJoint::default();
        let tractor_pos = Vec2::new(0.0, 0.0);
        // Exaggerated sharp tractor heading to test turntable stop
        let sharp_heading = 2.0; // > 110 degrees

        let state = joint.step(tractor_pos, sharp_heading, 5.0, 0.1);
        assert!(state.angle_rad <= joint.max_angle_rad);
        assert!((state.angle_rad - joint.max_angle_rad).abs() < 1e-4);
    }
}
