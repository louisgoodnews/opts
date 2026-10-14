//! Rigid body chassis dynamics and fixed-step numerical integration.
//!
//! Models 2D planar chassis movement (longitudinal translation and yaw rotation)
//! coupled with vertical elevation, velocity clamping, and coordinate projection.

use glam::Vec2;
use serde::{Deserialize, Serialize};

/// Dynamic rigid-body physical state of the vehicle chassis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChassisState {
    /// 2D horizontal world position in meters (X = lateral/East, Y = longitudinal/North).
    pub position: Vec2,
    /// Vertical elevation above road datum in meters.
    pub elevation_z: f32,
    /// Yaw heading rotation angle in radians (0.0 = pointing along +Y).
    pub heading_rad: f32,
    /// Linear velocity in the local vehicle frame in m/s (X = lateral slip velocity, Y = forward speed).
    pub local_velocity: Vec2,
    /// Angular yaw velocity around vertical axis in radians per second (+ = clockwise / right).
    pub yaw_rate_rad_s: f32,
    /// Linear acceleration in the local vehicle frame in m/s^2.
    pub local_acceleration: Vec2,
    /// Total curb weight plus passenger payload mass in kilograms.
    pub total_mass_kg: f32,
    /// Yaw mass moment of inertia around the vertical axis in kg·m^2.
    pub yaw_inertia_kg_m2: f32,
}

impl Default for ChassisState {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            elevation_z: 0.0,
            heading_rad: 0.0,
            local_velocity: Vec2::ZERO,
            yaw_rate_rad_s: 0.0,
            local_acceleration: Vec2::ZERO,
            total_mass_kg: 11_500.0, // Standard 12m curb weight (11.5 metric tons)
            yaw_inertia_kg_m2: 120_000.0, // Typical yaw inertia for a 12m transit bus
        }
    }
}

impl ChassisState {
    /// Creates a chassis state with specified total mass and world initial position.
    #[must_use]
    pub fn new(mass_kg: f32, initial_position: Vec2, initial_heading_rad: f32) -> Self {
        let mass = mass_kg.max(1_000.0);
        // Estimate yaw inertia: I_z ~ (1/12) * m * (width^2 + length^2)
        let estimated_inertia = (1.0 / 12.0) * mass * (2.5 * 2.5 + 12.0 * 12.0);

        Self {
            position: initial_position,
            elevation_z: 0.0,
            heading_rad: initial_heading_rad,
            local_velocity: Vec2::ZERO,
            yaw_rate_rad_s: 0.0,
            local_acceleration: Vec2::ZERO,
            total_mass_kg: mass,
            yaw_inertia_kg_m2: estimated_inertia,
        }
    }

    /// Forward speed along vehicle heading in meters per second.
    #[must_use]
    pub fn forward_speed_m_s(&self) -> f32 {
        self.local_velocity.y
    }

    /// Forward speed in kilometers per hour.
    #[must_use]
    pub fn forward_speed_km_h(&self) -> f32 {
        self.local_velocity.y * 3.6
    }

    /// Lateral tire slip velocity in meters per second.
    #[must_use]
    pub fn lateral_slip_m_s(&self) -> f32 {
        self.local_velocity.x
    }

    /// Linear velocity converted into the 2D world coordinate frame.
    #[must_use]
    pub fn world_velocity(&self) -> Vec2 {
        let sin_h = self.heading_rad.sin();
        let cos_h = self.heading_rad.cos();
        // Local X is lateral right (perpendicular), Local Y is forward
        Vec2::new(
            self.local_velocity.x * cos_h + self.local_velocity.y * sin_h,
            -self.local_velocity.x * sin_h + self.local_velocity.y * cos_h,
        )
    }

    /// Performs semi-implicit Euler numerical integration for a single fixed time step.
    ///
    /// # Arguments
    /// * `net_longitudinal_force_n` - Sum of tractive, braking, and resistance forces in Newtons.
    /// * `net_yaw_torque_nm` - Net turning moment around vertical axis in Newton-meters.
    /// * `is_braking` - Whether vehicle brakes are engaged (used for zero-speed clamping).
    /// * `dt` - Fixed integration step duration in seconds.
    pub fn integrate(
        &mut self,
        net_longitudinal_force_n: f32,
        net_yaw_torque_nm: f32,
        is_braking: bool,
        dt: f32,
    ) {
        self.integrate_with_mass(
            net_longitudinal_force_n,
            net_yaw_torque_nm,
            is_braking,
            dt,
            self.total_mass_kg,
        );
    }

    /// Like [`ChassisState::integrate`], but the longitudinal force acts on `moving_mass_kg`
    /// instead of the chassis' own mass: the mass of the whole consist the chassis tows
    /// (`Σ mᵢ`, see `consist`). Lateral damping and yaw keep the chassis' own mass and inertia.
    pub fn integrate_with_mass(
        &mut self,
        net_longitudinal_force_n: f32,
        net_yaw_torque_nm: f32,
        is_braking: bool,
        dt: f32,
        moving_mass_kg: f32,
    ) {
        if dt <= 0.0 {
            return;
        }

        // 1. Calculate accelerations
        let accel_y = net_longitudinal_force_n / moving_mass_kg.max(1.0);
        // High lateral tire damping keeps lateral slip negligible under standard road conditions
        let lateral_damping_force = -self.local_velocity.x * (self.total_mass_kg * 8.0);
        let accel_x = lateral_damping_force / self.total_mass_kg;

        self.local_acceleration = Vec2::new(accel_x, accel_y);

        let yaw_accel = net_yaw_torque_nm / self.yaw_inertia_kg_m2;

        // 2. Semi-implicit Euler integration for velocities
        let prev_v_y = self.local_velocity.y;
        self.local_velocity.y += accel_y * dt;
        self.local_velocity.x += accel_x * dt;
        self.yaw_rate_rad_s += yaw_accel * dt;

        // 3. Defensive zero-speed clamping to eliminate stationary numerical vibration
        if is_braking
            && (self.local_velocity.y.abs() < 0.1
                || (prev_v_y > 0.0 && self.local_velocity.y < 0.0)
                || (prev_v_y < 0.0 && self.local_velocity.y > 0.0))
        {
            self.local_velocity.y = 0.0;
            self.local_acceleration.y = 0.0;
        }
        if self.local_velocity.x.abs() < 0.01 {
            self.local_velocity.x = 0.0;
        }
        if self.yaw_rate_rad_s.abs() < 0.001 && self.local_velocity.y.abs() < 0.05 {
            self.yaw_rate_rad_s = 0.0;
        }

        // 4. Update heading and position
        self.heading_rad += self.yaw_rate_rad_s * dt;
        let world_v = self.world_velocity();
        self.position += world_v * dt;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chassis_acceleration_and_position() {
        let mut chassis = ChassisState::new(10_000.0, Vec2::ZERO, 0.0);
        let force_10kn = 10_000.0; // a = 1.0 m/s^2

        for _ in 0..10 {
            chassis.integrate(force_10kn, 0.0, false, 0.1);
        }

        // After 1.0 second at a = 1.0 m/s^2, speed ~ 1.0 m/s
        assert!((chassis.forward_speed_m_s() - 1.0).abs() < 0.05);
        // Position along Y axis must be positive
        assert!(chassis.position.y > 0.0);
    }

    #[test]
    fn test_zero_speed_clamping_under_braking() {
        let mut chassis = ChassisState::new(10_000.0, Vec2::ZERO, 0.0);
        chassis.local_velocity = Vec2::new(0.0, 0.02); // 2 cm/s residual creep

        // Braking with small negative force
        chassis.integrate(-500.0, 0.0, true, 0.1);

        // Clamping must bring speed exactly to 0.0
        assert_eq!(chassis.forward_speed_m_s(), 0.0);
    }
}
