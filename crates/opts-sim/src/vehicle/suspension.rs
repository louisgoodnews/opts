//! Suspension spring-damper compliance and dynamic load transfer across axles.
//!
//! Models static axle load distribution based on center-of-gravity placement,
//! dynamic longitudinal weight transfer during acceleration and braking, and
//! suspension ride-height deflection.

use crate::vehicle::axle::{AxleConfig, AxleState};
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

/// Travel of a spring (m) beyond which the bump stop acts (assumed, the files state no travel ❓).
pub const BUMP_STOP_TRAVEL_M: f32 = 0.12;
/// Stiffness of the bump stop (N/m).
const BUMP_STOP_STIFFNESS_N_M: f32 = 2.0e6;
/// Integration substep of the body model (s).
const BODY_SUBSTEP_S: f32 = 0.004;

/// Gravity (m/s²).
const GRAVITY: f32 = 9.81;

fn one() -> f32 {
    1.0
}

/// Lowest and highest factor on a spring the physics accepts (a script bug must not turn the
/// body into a rock or a jelly).
pub const SPRING_FACTOR_RANGE: (f32, f32) = (0.2, 5.0);

/// One wheel's spring and damper under the body.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WheelSuspension {
    /// Position under the body: longitudinal (m, forward positive) and lateral (m, right positive).
    pub x_m: f32,
    /// Lateral position (m).
    pub y_m: f32,
    /// Spring rate of this wheel (N/m).
    pub spring_n_m: f32,
    /// Damping of this wheel (N·s/m).
    pub damping_n_s_m: f32,
    /// Factor on the spring rate the scripts write (`Axle_Springfactor_<n>_<l|r>`, air
    /// suspension and kneeling); 1.0 is the file's spring.
    #[serde(default = "one")]
    pub spring_factor: f32,
    /// Load at rest (N).
    pub static_load_n: f32,
    /// Spring compression beyond the rest position (m, compression positive).
    pub compression_m: f32,
    /// Rate of the compression (m/s).
    pub compression_rate_m_s: f32,
    /// Vertical load on the ground now (N, zero when the wheel has lifted).
    pub load_n: f32,
    /// The wheel's share of the weight of the people on board (N): a downward force on the body
    /// at the wheel's position (M-0124); the body sinks until the springs carry it.
    #[serde(default)]
    pub payload_n: f32,
}

/// The body on its wheels: heave, pitch and roll under a spring and a damper per wheel, driven by
/// the longitudinal and lateral acceleration (their moments about the centre of gravity) and the
/// ground under each wheel (M-0073). Small angles; the rest position is the static equilibrium of
/// the loads the [`SuspensionModel`] distributes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BodySuspension {
    /// Height of the body above its rest position (m, up positive).
    pub heave_m: f32,
    /// Heave rate (m/s).
    pub heave_rate_m_s: f32,
    /// Pitch (rad, nose up positive).
    pub pitch_rad: f32,
    /// Pitch rate (rad/s).
    pub pitch_rate_rad_s: f32,
    /// Roll (rad, right side down positive).
    pub roll_rad: f32,
    /// Roll rate (rad/s).
    pub roll_rate_rad_s: f32,
    /// The wheels, axle by axle, left then right.
    pub wheels: Vec<WheelSuspension>,
    /// Sprung mass (kg).
    pub mass_kg: f32,
    /// Pitch moment of inertia about the lateral axis (kg·m²).
    pub pitch_inertia_kg_m2: f32,
    /// Roll moment of inertia about the longitudinal axis (kg·m²).
    pub roll_inertia_kg_m2: f32,
    /// Height of the centre of gravity (m).
    pub cg_height_m: f32,
    /// Vertical acceleration of the body on its springs over the last step (m/s², up
    /// positive): what the scripts' `A_Trans_Z` adds to the ground's (M-0146).
    #[serde(default)]
    pub vertical_acc_m_s2: f32,
}

impl BodySuspension {
    /// A body at rest on the wheels of `axles` (longitudinal position, track width, spring rate
    /// and damping per side), carrying the static axle loads `static_loads_n` (summing to the
    /// weight).
    #[must_use]
    pub fn new(
        axles: &[AxleConfig],
        static_loads_n: &[f32],
        mass_kg: f32,
        pitch_inertia_kg_m2: f32,
        roll_inertia_kg_m2: f32,
        cg_height_m: f32,
    ) -> Self {
        let mut wheels = Vec::with_capacity(axles.len() * 2);
        for (axle, load) in axles.iter().zip(static_loads_n) {
            for side in [-1.0f32, 1.0] {
                wheels.push(WheelSuspension {
                    x_m: axle.longitudinal_m,
                    y_m: side * axle.track_width_m * 0.5,
                    spring_n_m: axle.spring_rate_n_m.max(10_000.0),
                    damping_n_s_m: axle.damping_n_s_m.max(1_000.0),
                    spring_factor: 1.0,
                    static_load_n: load * 0.5,
                    compression_m: 0.0,
                    compression_rate_m_s: 0.0,
                    load_n: load * 0.5,
                    payload_n: 0.0,
                });
            }
        }
        Self {
            heave_m: 0.0,
            heave_rate_m_s: 0.0,
            pitch_rad: 0.0,
            pitch_rate_rad_s: 0.0,
            roll_rad: 0.0,
            roll_rate_rad_s: 0.0,
            wheels,
            mass_kg: mass_kg.max(100.0),
            pitch_inertia_kg_m2: pitch_inertia_kg_m2.max(100.0),
            roll_inertia_kg_m2: roll_inertia_kg_m2.max(100.0),
            cg_height_m: cg_height_m.max(0.1),
            vertical_acc_m_s2: 0.0,
        }
    }

    /// Advances `dt` seconds. `a_long` is the longitudinal acceleration (m/s², forward positive),
    /// `a_lat` the lateral one (m/s², towards the right positive); `ground_m` the height of the
    /// ground under each wheel against the plane the body follows (m, same order as
    /// [`BodySuspension::wheels`]; missing entries are 0).
    pub fn step(&mut self, dt: f32, a_long: f32, a_lat: f32, ground_m: &[f32]) {
        if dt <= 0.0 {
            return;
        }
        let n = (dt / BODY_SUBSTEP_S).ceil().max(1.0) as usize;
        let h = dt / n as f32;
        // The people on board add mass and inertia (their weight sits at the wheels' places).
        let (mut m, mut ip, mut ir) = (
            self.mass_kg,
            self.pitch_inertia_kg_m2,
            self.roll_inertia_kg_m2,
        );
        for w in &self.wheels {
            let pm = w.payload_n / GRAVITY;
            m += pm;
            ip += pm * w.x_m * w.x_m;
            ir += pm * w.y_m * w.y_m;
        }
        // The d'Alembert forces of the acceleration act at the centre of gravity: speeding up
        // lifts the nose, turning right rolls the body to the left (right side up).
        let m_pitch_ext = m * a_long * self.cg_height_m;
        let m_roll_ext = -m * a_lat * self.cg_height_m;
        let mut heave_acc_sum = 0.0f32;
        for _ in 0..n {
            let (mut fz, mut mp, mut mr) = (0.0f32, 0.0f32, 0.0f32);
            for (i, w) in self.wheels.iter_mut().enumerate() {
                let g = ground_m.get(i).copied().unwrap_or(0.0);
                // Height of the body above this wheel's rest position, then the compression.
                let body = self.heave_m + w.x_m * self.pitch_rad - w.y_m * self.roll_rad;
                let body_rate = self.heave_rate_m_s + w.x_m * self.pitch_rate_rad_s
                    - w.y_m * self.roll_rate_rad_s;
                let compression = -body + g;
                let rate = -body_rate;
                // The factor scales the whole spring, the rest load included: the body rests
                // lower on a softer spring (kneeling, `Axle_Springfactor` 0.55) and higher on a
                // stiffer one (raising to 2), where scaling only the compression would change
                // the stiffness and never the height.
                let mut force = w.spring_n_m * w.spring_factor * compression
                    + (w.spring_factor - 1.0) * w.static_load_n
                    + w.damping_n_s_m * rate;
                if compression > BUMP_STOP_TRAVEL_M {
                    force += BUMP_STOP_STIFFNESS_N_M * (compression - BUMP_STOP_TRAVEL_M);
                }
                // A wheel cannot pull: at most it loses all its load.
                let load = (w.static_load_n + force).max(0.0);
                let delta = load - w.static_load_n;
                w.compression_m = compression;
                w.compression_rate_m_s = rate;
                w.load_n = load;
                // The spring's deviation lifts the body, the payload's weight pulls it down.
                let net = delta - w.payload_n;
                fz += net;
                mp += w.x_m * net;
                mr -= w.y_m * net;
            }
            let heave_acc = fz / m;
            heave_acc_sum += heave_acc;
            let pitch_acc = (mp + m_pitch_ext) / ip;
            let roll_acc = (mr + m_roll_ext) / ir;
            self.heave_rate_m_s += heave_acc * h;
            self.pitch_rate_rad_s += pitch_acc * h;
            self.roll_rate_rad_s += roll_acc * h;
            self.heave_m += self.heave_rate_m_s * h;
            self.pitch_rad += self.pitch_rate_rad_s * h;
            self.roll_rad += self.roll_rate_rad_s * h;
        }
        self.vertical_acc_m_s2 = heave_acc_sum / n as f32;
    }

    /// Sets the weight of the people on board (N per axle, left and right wheel half each).
    pub fn set_payload(&mut self, axle_loads_n: &[f32]) {
        for (i, w) in self.wheels.iter_mut().enumerate() {
            w.payload_n = axle_loads_n.get(i / 2).copied().unwrap_or(0.0) * 0.5;
        }
    }

    /// The weight of the people on board (N).
    #[must_use]
    pub fn payload_n(&self) -> f32 {
        self.wheels.iter().map(|w| w.payload_n).sum()
    }

    /// Sum of the loads on the wheels of axle `index` (N).
    #[must_use]
    pub fn axle_load_n(&self, index: usize) -> f32 {
        self.wheels
            .get(index * 2..index * 2 + 2)
            .map_or(0.0, |w| w.iter().map(|w| w.load_n).sum())
    }

    /// Sum of all wheel loads (N).
    #[must_use]
    pub fn total_load_n(&self) -> f32 {
        self.wheels.iter().map(|w| w.load_n).sum()
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
