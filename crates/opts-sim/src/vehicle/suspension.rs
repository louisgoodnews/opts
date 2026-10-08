//! Suspension spring-damper compliance and dynamic load transfer across axles.
//!
//! Models static axle load distribution based on center-of-gravity placement,
//! dynamic longitudinal weight transfer during acceleration and braking, and
//! suspension ride-height deflection.

use crate::vehicle::axle::AxleState;
use serde::{Deserialize, Serialize};

/// High-level suspension and mass distribution calculator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SuspensionModel {
    /// Height of the vehicle center of gravity above the road in meters.
    pub cg_height_m: f32,
    /// Longitudinal position of the center of gravity along the vehicle coordinate axis (+Y = forward).
    pub cg_longitudinal_m: f32,
}

impl Default for SuspensionModel {
    fn default() -> Self {
        Self {
            cg_height_m: 1.15, // Standard low-floor transit bus center of gravity
            cg_longitudinal_m: 0.0,
        }
    }
}

impl SuspensionModel {
    /// Creates a suspension model with specified center of gravity coordinates.
    #[must_use]
    pub fn new(cg_height_m: f32, cg_longitudinal_m: f32) -> Self {
        Self {
            cg_height_m: cg_height_m.max(0.2),
            cg_longitudinal_m,
        }
    }

    /// Computes static vertical normal loads for each axle at rest in equilibrium.
    ///
    /// # Arguments
    /// * `axle_positions` - Longitudinal Y coordinate of each axle in meters.
    /// * `vehicle_mass_kg` - Total vehicle curb weight in kilograms.
    ///
    /// # Returns
    /// A vector of static normal forces in Newtons, one per axle.
    #[must_use]
    pub fn compute_static_loads(&self, axle_positions: &[f32], vehicle_mass_kg: f32) -> Vec<f32> {
        let g = 9.81;
        let total_weight_n = vehicle_mass_kg.max(100.0) * g;
        let count = axle_positions.len();

        if count == 0 {
            return Vec::new();
        }
        if count == 1 {
            return vec![total_weight_n];
        }

        // For a typical 2-axle vehicle: Axle 0 (front) and Axle 1 (rear)
        if count == 2 {
            let y_front = axle_positions[0];
            let y_rear = axle_positions[1];
            let wheelbase = (y_front - y_rear).abs().max(1.0);

            // Distance from each axle to CG
            let d_front = (y_front - self.cg_longitudinal_m).abs();
            let d_rear = (self.cg_longitudinal_m - y_rear).abs();

            let front_ratio = (d_rear / wheelbase).clamp(0.2, 0.8);
            let rear_ratio = (d_front / wheelbase).clamp(0.2, 0.8);

            return vec![total_weight_n * front_ratio, total_weight_n * rear_ratio];
        }

        // Multi-axle (3+ axles): distribute inversely proportional to distance from CG
        let distances: Vec<f32> = axle_positions
            .iter()
            .map(|&y| (y - self.cg_longitudinal_m).abs().max(0.5))
            .collect();

        let inv_sum: f32 = distances.iter().map(|&d| 1.0 / d).sum();
        if inv_sum > 0.0 {
            distances
                .iter()
                .map(|&d| total_weight_n * ((1.0 / d) / inv_sum))
                .collect()
        } else {
            vec![total_weight_n / (count as f32); count]
        }
    }

    /// Computes dynamic normal loads across axles under longitudinal acceleration or braking.
    ///
    /// # Arguments
    /// * `static_loads` - Static axle normal loads in Newtons.
    /// * `axle_positions` - Longitudinal coordinates of the axles in meters.
    /// * `mass_kg` - Vehicle mass in kilograms.
    /// * `accel_long_m_s2` - Longitudinal acceleration in m/s^2 (+ = forward acceleration, - = braking).
    ///
    /// # Returns
    /// Dynamic normal loads in Newtons for each axle.
    #[must_use]
    pub fn compute_dynamic_loads(
        &self,
        static_loads: &[f32],
        axle_positions: &[f32],
        mass_kg: f32,
        accel_long_m_s2: f32,
    ) -> Vec<f32> {
        let count = static_loads.len();
        if count < 2 || axle_positions.len() < 2 {
            return static_loads.to_vec();
        }

        let y_front = axle_positions[0];
        let y_rear = axle_positions[count - 1];
        let wheelbase = (y_front - y_rear).abs().max(1.0);

        // Dynamic load transfer: Delta_W = m * a * h_cg / L
        let delta_w = mass_kg * accel_long_m_s2 * (self.cg_height_m / wheelbase);

        let mut dynamic = static_loads.to_vec();
        // Acceleration (a > 0): removes load from front, adds to rear
        // Braking (a < 0): adds load to front, removes from rear
        dynamic[0] = (dynamic[0] - delta_w).max(0.0);
        dynamic[count - 1] = (dynamic[count - 1] + delta_w).max(0.0);

        dynamic
    }

    /// Updates axle suspension vertical deflection and wheel normal loads based on dynamic force.
    pub fn update_axle_suspension(
        &self,
        axle: &mut AxleState,
        static_load_n: f32,
        dynamic_load_n: f32,
        dt: f32,
    ) {
        let delta_force = dynamic_load_n - static_load_n;
        let k = axle.config.spring_rate_n_m.max(10_000.0);
        let c = axle.config.damping_n_s_m.max(1_000.0);

        let target_deflection = delta_force / k;
        let diff = target_deflection - axle.suspension_deflection_m;

        // Spring-damper relaxation towards target deflection
        let velocity = (diff / dt.max(0.001)).clamp(-2.0, 2.0);
        axle.suspension_velocity_m_s = velocity;
        axle.suspension_deflection_m += velocity * dt * (1.0 - (-c * dt / 1000.0).exp().min(0.9));

        // Distribute load equally between left and right wheels
        let per_wheel_load = dynamic_load_n * 0.5;
        axle.left_wheel.vertical_load_n = per_wheel_load;
        axle.right_wheel.vertical_load_n = per_wheel_load;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_load_sum_equals_weight() {
        let susp = SuspensionModel::new(1.15, 0.0);
        let mass = 12_000.0; // 12 metric tons
        let total_weight = mass * 9.81;

        let axle_positions = [3.2, -2.6]; // Front axle +3.2m, Rear axle -2.6m
        let loads = susp.compute_static_loads(&axle_positions, mass);

        let sum: f32 = loads.iter().sum();
        assert!(
            (sum - total_weight).abs() < 1.0,
            "Sum of axle loads must equal total weight"
        );
        assert!(loads[0] > 0.0);
        assert!(loads[1] > 0.0);
    }

    #[test]
    fn test_dynamic_load_transfer() {
        let susp = SuspensionModel::new(1.15, 0.0);
        let mass = 12_000.0;
        let axle_positions = [3.2, -2.6];
        let static_loads = susp.compute_static_loads(&axle_positions, mass);

        // 1. Forward acceleration (+2 m/s^2) shifts weight to rear
        let accel_loads = susp.compute_dynamic_loads(&static_loads, &axle_positions, mass, 2.0);
        assert!(
            accel_loads[1] > static_loads[1],
            "Rear axle must gain weight on acceleration"
        );
        assert!(
            accel_loads[0] < static_loads[0],
            "Front axle must lose weight on acceleration"
        );

        // 2. Heavy braking (-5 m/s^2) shifts weight to front
        let brake_loads = susp.compute_dynamic_loads(&static_loads, &axle_positions, mass, -5.0);
        assert!(
            brake_loads[0] > static_loads[0],
            "Front axle must gain weight on braking"
        );
        assert!(
            brake_loads[1] < static_loads[1],
            "Rear axle must lose weight on braking"
        );
    }
}
