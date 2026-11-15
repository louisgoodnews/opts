//! The static configuration and limits of an axle.

use serde::{Deserialize, Serialize};

/// Static mechanical configuration and limits of an axle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AxleConfig {
    /// Longitudinal position relative to vehicle coordinate origin in meters (+Y = forward).
    pub longitudinal_m: f32,
    /// Wheel track width between left and right tire centers in meters.
    pub track_width_m: f32,
    /// Nominal outer wheel tire diameter in meters.
    pub wheel_diameter_m: f32,
    /// Suspension spring stiffness rate in Newtons per meter.
    pub spring_rate_n_m: f32,
    /// Maximum suspension travel force rating in Newtons.
    pub max_force_n: f32,
    /// Shock absorber damping rate in Newton-seconds per meter.
    pub damping_n_s_m: f32,
    /// Whether this axle receives tractive torque from the powertrain.
    pub is_driven: bool,
    /// Whether this axle steers the vehicle.
    pub is_steered: bool,
    /// Maximum mechanical road wheel steering angle limit in radians.
    pub max_steer_angle_rad: f32,
}

impl Default for AxleConfig {
    fn default() -> Self {
        Self {
            longitudinal_m: 0.0,
            track_width_m: 2.45,
            wheel_diameter_m: 0.98,
            spring_rate_n_m: 250_000.0,
            max_force_n: 115_000.0,
            damping_n_s_m: 15_000.0,
            is_driven: false,
            is_steered: false,
            max_steer_angle_rad: 0.82, // ~47 degrees
        }
    }
}
