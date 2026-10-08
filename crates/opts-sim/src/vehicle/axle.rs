//! Axle and wheel kinematics, Ackermann geometry, and rotation states.
//!
//! Models physical vehicle axles, dual wheels (left and right), suspension
//! mounting parameters, and differential Ackermann steering angles.

use opts_core::formats::vehicles::bus::BusAxle;
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

/// Dynamic state and kinematics of an axle assembly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AxleState {
    /// Mechanical configuration parameters.
    pub config: AxleConfig,
    /// State of the left road wheel.
    pub left_wheel: WheelState,
    /// State of the right road wheel.
    pub right_wheel: WheelState,
    /// Vertical suspension displacement from static equilibrium in meters (+ = bump/compression).
    pub suspension_deflection_m: f32,
    /// Suspension vertical velocity in meters per second.
    pub suspension_velocity_m_s: f32,
}

impl AxleState {
    /// Constructs an axle state from a parsed [`BusAxle`] specification.
    ///
    /// Provides physically realistic commercial vehicle fallbacks for optional manifest fields.
    #[must_use]
    pub fn from_manifest(axle: &BusAxle, is_steered: bool) -> Self {
        let wheel_diameter = axle.wheel_diameter_m.filter(|&d| d > 0.1).unwrap_or(0.98); // Standard 275/70 R22.5 transit tire is ~0.96-0.98m

        let track_width = axle
            .max_width_m
            .or(axle.min_width_m)
            .filter(|&w| w > 0.5)
            .unwrap_or(2.45);

        // Convert OMSI kN/m into N/m
        let spring_rate = axle
            .spring_rate
            .filter(|&s| s > 0.0)
            .map_or(250_000.0, |s| s * 1_000.0);
        let max_force = axle
            .max_force
            .filter(|&f| f > 0.0)
            .map_or(115_000.0, |f| f * 1_000.0);
        let damping = axle
            .damping
            .filter(|&d| d > 0.0)
            .map_or(15_000.0, |d| d * 1_000.0);

        let config = AxleConfig {
            longitudinal_m: axle.longitudinal_m,
            track_width_m: track_width,
            wheel_diameter_m: wheel_diameter,
            spring_rate_n_m: spring_rate,
            max_force_n: max_force,
            damping_n_s_m: damping,
            is_driven: axle.is_driven,
            is_steered,
            max_steer_angle_rad: if is_steered { 0.82 } else { 0.0 },
        };

        let rolling_radius = wheel_diameter * 0.5;
        let left_wheel = WheelState {
            rolling_radius_m: rolling_radius,
            ..Default::default()
        };
        let right_wheel = WheelState {
            rolling_radius_m: rolling_radius,
            ..Default::default()
        };

        Self {
            config,
            left_wheel,
            right_wheel,
            suspension_deflection_m: 0.0,
            suspension_velocity_m_s: 0.0,
        }
    }

    /// Computes Ackermann steering angles for the left and right wheels given a central steering angle.
    ///
    /// # Arguments
    /// * `wheelbase_m` - Distance between this steering axle and the effective rear rotation center in meters.
    /// * `track_width_m` - Lateral distance between left and right wheel hubs in meters.
    /// * `central_angle_rad` - Virtual centerline road wheel steering angle in radians (negative = left, positive = right).
    ///
    /// # Returns
    /// A tuple of `(left_angle_rad, right_angle_rad)`.
    #[must_use]
    pub fn compute_ackermann_angles(
        wheelbase_m: f32,
        track_width_m: f32,
        central_angle_rad: f32,
    ) -> (f32, f32) {
        // Defensive check: If steering is effectively straight ahead, avoid division by zero
        if central_angle_rad.abs() < 1e-6 || wheelbase_m.abs() < 1e-4 {
            return (central_angle_rad, central_angle_rad);
        }

        let tan_delta = central_angle_rad.tan();
        if tan_delta.abs() < 1e-6 {
            return (0.0, 0.0);
        }

        // Instantaneous center of rotation distance from vehicle centerline
        let r_center = wheelbase_m / tan_delta;
        let half_track = track_width_m * 0.5;

        // When turning right (central_angle_rad > 0):
        // Right wheel is inner: R_inner = R_center - W/2
        // Left wheel is outer:  R_outer = R_center + W/2
        let r_left = r_center + half_track;
        let r_right = r_center - half_track;

        let left_angle = if r_left.abs() > 1e-4 {
            (wheelbase_m / r_left).atan()
        } else {
            central_angle_rad
        };

        let right_angle = if r_right.abs() > 1e-4 {
            (wheelbase_m / r_right).atan()
        } else {
            central_angle_rad
        };

        (left_angle, right_angle)
    }

    /// Updates the steering angles of the axle's wheels based on driver input and wheelbase.
    pub fn apply_steering(&mut self, central_angle_rad: f32, wheelbase_m: f32) {
        if !self.config.is_steered {
            self.left_wheel.steer_angle_rad = 0.0;
            self.right_wheel.steer_angle_rad = 0.0;
            return;
        }

        let clamped_central = central_angle_rad.clamp(
            -self.config.max_steer_angle_rad,
            self.config.max_steer_angle_rad,
        );

        let (left, right) =
            Self::compute_ackermann_angles(wheelbase_m, self.config.track_width_m, clamped_central);

        self.left_wheel.steer_angle_rad = left;
        self.right_wheel.steer_angle_rad = right;
    }

    /// Advances wheel rotation angles based on longitudinal forward velocity.
    pub fn update_rotation(&mut self, forward_velocity_m_s: f32, dt: f32) {
        let radius = (self.config.wheel_diameter_m * 0.5).max(0.1);
        let ang_vel = forward_velocity_m_s / radius;

        self.left_wheel.angular_velocity_rad_s = ang_vel;
        self.right_wheel.angular_velocity_rad_s = ang_vel;

        self.left_wheel.rotation_rad =
            (self.left_wheel.rotation_rad + ang_vel * dt) % (std::f32::consts::TAU);
        self.right_wheel.rotation_rad =
            (self.right_wheel.rotation_rad + ang_vel * dt) % (std::f32::consts::TAU);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ackermann_straight_line_safe() {
        // Zero steering angle must return (0.0, 0.0) without NaN or panic
        let (left, right) = AxleState::compute_ackermann_angles(5.8, 2.45, 0.0);
        assert_eq!(left, 0.0);
        assert_eq!(right, 0.0);
        assert!(!left.is_nan());
        assert!(!right.is_nan());
    }

    #[test]
    fn test_ackermann_steering_geometry() {
        let wheelbase = 5.85; // Standard 12m bus wheelbase
        let track_width = 2.45;
        let steer_input = 0.5; // ~28.6 degrees right turn

        let (left_angle, right_angle) =
            AxleState::compute_ackermann_angles(wheelbase, track_width, steer_input);

        // For a right turn: right wheel is inner, left wheel is outer.
        // Inner wheel must turn sharper than outer wheel: right_angle > left_angle > 0
        assert!(
            right_angle > left_angle,
            "Right inner wheel must turn sharper than left outer"
        );
        assert!(left_angle > 0.0);
        assert!(right_angle > 0.0);
    }

    #[test]
    fn test_wheel_rotation_advancement() {
        let mut axle = AxleState::from_manifest(
            &BusAxle {
                longitudinal_m: 3.2,
                max_width_m: Some(2.45),
                min_width_m: None,
                wheel_diameter_m: Some(1.0), // r = 0.5m
                spring_rate: None,
                max_force: None,
                damping: None,
                is_driven: false,
            },
            true,
        );

        // At 10 m/s with r = 0.5m, ang_vel = 20 rad/s
        axle.update_rotation(10.0, 0.1);
        assert!((axle.left_wheel.angular_velocity_rad_s - 20.0).abs() < 1e-3);
        assert!((axle.left_wheel.rotation_rad - 2.0).abs() < 1e-3);
    }
}
