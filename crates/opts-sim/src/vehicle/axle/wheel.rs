//! The state of one road wheel: rotation, steering, rolling radius and load.

use serde::{Deserialize, Serialize};

/// Physical state of an individual road wheel.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WheelState {
    /// Cumulative rotation angle around the spin axis in radians (used for mesh spinning).
    pub rotation_rad: f32,
    /// Rotational angular velocity in radians per second.
    pub angular_velocity_rad_s: f32,
    /// Current steering turn angle relative to the chassis centerline in radians.
    pub steer_angle_rad: f32,
    /// Effective dynamic rolling radius in meters.
    pub rolling_radius_m: f32,
    /// Vertical normal contact force in Newtons.
    pub vertical_load_n: f32,
}

impl Default for WheelState {
    fn default() -> Self {
        Self {
            rotation_rad: 0.0,
            angular_velocity_rad_s: 0.0,
            steer_angle_rad: 0.0,
            rolling_radius_m: 0.49, // ~0.98m wheel diameter default
            vertical_load_n: 0.0,
        }
    }
}
