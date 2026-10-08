//! Vehicle physics, kinematics, and powertrain simulation.
//!
//! Provides the primary [`VehicleSimulation`] controller orchestrating multi-axle
//! Ackermann steering, longitudinal propulsion, braking systems, dynamic suspension
//! compliance, and optional multi-unit trailer articulation.

pub mod articulation;
pub mod axle;
pub mod chassis;
pub mod drivetrain;
pub mod inputs;
pub mod suspension;

use self::articulation::ArticulationJoint;
use self::axle::AxleState;
use self::chassis::ChassisState;
use self::drivetrain::DrivetrainModel;
use self::inputs::VehicleInputs;
use self::suspension::SuspensionModel;

use glam::Vec2;
use opts_core::formats::vehicles::bus::BusManifest;
use serde::{Deserialize, Serialize};

/// High-level vehicle physics simulation orchestrator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VehicleSimulation {
    /// Rigid body chassis state and coordinates.
    pub chassis: ChassisState,
    /// Ordered list of vehicle axles and road wheels.
    pub axles: Vec<AxleState>,
    /// Powertrain, gearing, and braking system.
    pub drivetrain: DrivetrainModel,
    /// Vertical suspension and load distribution model.
    pub suspension: SuspensionModel,
    /// Optional articulated trailer joint coupling.
    pub articulation: Option<ArticulationJoint>,
    /// Active driver control inputs.
    pub inputs: VehicleInputs,
    /// Effective wheelbase between front steering axle and rear reference axle in meters.
    pub wheelbase_m: f32,
}

impl Default for VehicleSimulation {
    fn default() -> Self {
        Self::new_standard_bus(11_500.0, 5.85)
    }
}

impl VehicleSimulation {
    /// Constructs a standard two-axle commercial transit bus simulation.
    #[must_use]
    pub fn new_standard_bus(mass_kg: f32, wheelbase_m: f32) -> Self {
        let wheelbase = wheelbase_m.max(2.0);
        let half_base = wheelbase * 0.5;

        // Front steer axle at +half_base, Rear driven axle at -half_base
        let mut front_axle = AxleState::from_manifest(
            &opts_core::formats::vehicles::bus::BusAxle {
                longitudinal_m: half_base,
                max_width_m: Some(2.45),
                min_width_m: None,
                wheel_diameter_m: Some(0.98),
                spring_rate: None,
                max_force: None,
                damping: None,
                is_driven: false,
            },
            true,
        );
        front_axle.config.is_steered = true;

        let rear_axle = AxleState::from_manifest(
            &opts_core::formats::vehicles::bus::BusAxle {
                longitudinal_m: -half_base,
                max_width_m: Some(2.45),
                min_width_m: None,
                wheel_diameter_m: Some(0.98),
                spring_rate: None,
                max_force: None,
                damping: None,
                is_driven: true,
            },
            false,
        );

        let chassis = ChassisState::new(mass_kg, Vec2::ZERO, 0.0);
        let suspension = SuspensionModel::new(1.15, 0.0);

        Self {
            chassis,
            axles: vec![front_axle, rear_axle],
            drivetrain: DrivetrainModel::new(),
            suspension,
            articulation: None,
            inputs: VehicleInputs::new(),
            wheelbase_m: wheelbase,
        }
    }

    /// Constructs a full vehicle simulation from a parsed OMSI [`BusManifest`].
    #[must_use]
    pub fn from_manifest(manifest: &BusManifest) -> Self {
        let mass_kg = manifest
            .mass_tonnes
            .filter(|&m| m > 0.5)
            .map_or(11_500.0, |m| m * 1_000.0);

        let mut axles = Vec::new();

        for (idx, manifest_axle) in manifest.axles.iter().enumerate() {
            // By convention in OMSI manifests, the first axle (index 0) is the steer axle
            let is_steered = idx == 0;
            axles.push(AxleState::from_manifest(manifest_axle, is_steered));
        }

        // If manifest had no axles, supply default front and rear axles
        if axles.is_empty() {
            return Self::new_standard_bus(mass_kg, 5.85);
        }

        let y_front = axles.first().map_or(3.0, |a| a.config.longitudinal_m);
        let y_rear = axles.last().map_or(-3.0, |a| a.config.longitudinal_m);
        let wheelbase = (y_front - y_rear).abs().max(1.0);

        let articulation = manifest
            .coupling_back_y
            .map(|coupling_y| ArticulationJoint::new(coupling_y, 1.2, 6.0));

        Self {
            chassis: ChassisState::new(mass_kg, Vec2::ZERO, 0.0),
            axles,
            drivetrain: DrivetrainModel::new(),
            suspension: SuspensionModel::new(1.15, 0.0),
            articulation,
            inputs: VehicleInputs::new(),
            wheelbase_m: wheelbase,
        }
    }

    /// Steps the vehicle simulation by a fixed time delta `dt`.
    pub fn step(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }

        let speed = self.chassis.forward_speed_m_s();
        let first_wheel_radius = self
            .axles
            .first()
            .map_or(0.49, |a| a.config.wheel_diameter_m * 0.5);

        // 1. Steering kinematics: update all steered axles
        let central_steer_rad = self.inputs.steering() * 0.82; // Max ~47 degrees lock
        for axle in &mut self.axles {
            axle.apply_steering(central_steer_rad, self.wheelbase_m);
        }

        // 2. Drivetrain & braking forces
        let forces = self.drivetrain.step(
            &self.inputs,
            speed,
            first_wheel_radius,
            self.chassis.total_mass_kg,
        );

        let net_longitudinal_force =
            if speed.abs() < 0.01 && !self.inputs.is_braking() && forces.tractive_force_n == 0.0 {
                0.0
            } else {
                // Braking always opposes velocity direction
                let brake_dir = if speed > 0.001 {
                    -1.0
                } else if speed < -0.001 {
                    1.0
                } else if forces.tractive_force_n > 0.0 {
                    -1.0
                } else {
                    1.0
                };
                let drag_dir = if speed > 0.0 { -1.0 } else { 1.0 };

                forces.tractive_force_n
                    + (forces.braking_force_n * brake_dir)
                    + (forces.drag_force_n * drag_dir)
                    + (forces.rolling_resistance_n * drag_dir)
            };

        // 3. Kinematic turning yaw rate: omega = (v / L) * tan(delta)
        let yaw_rate = (speed / self.wheelbase_m) * central_steer_rad.tan();
        let yaw_torque =
            (yaw_rate - self.chassis.yaw_rate_rad_s) * self.chassis.yaw_inertia_kg_m2 * 10.0;

        // 4. Integrate chassis dynamics
        self.chassis.integrate(
            net_longitudinal_force,
            yaw_torque,
            self.inputs.is_braking(),
            dt,
        );

        // 5. Axle rotation and suspension load compliance
        let axle_positions: Vec<f32> = self.axles.iter().map(|a| a.config.longitudinal_m).collect();
        let static_loads = self
            .suspension
            .compute_static_loads(&axle_positions, self.chassis.total_mass_kg);
        let dynamic_loads = self.suspension.compute_dynamic_loads(
            &static_loads,
            &axle_positions,
            self.chassis.total_mass_kg,
            self.chassis.local_acceleration.y,
        );

        for (idx, axle) in self.axles.iter_mut().enumerate() {
            axle.update_rotation(self.chassis.forward_speed_m_s(), dt);
            let static_n = static_loads.get(idx).copied().unwrap_or(0.0);
            let dynamic_n = dynamic_loads.get(idx).copied().unwrap_or(0.0);
            self.suspension
                .update_axle_suspension(axle, static_n, dynamic_n, dt);
        }

        // 6. Articulation kinematics
        if let Some(ref mut joint) = self.articulation {
            let _ = joint.step(
                self.chassis.position,
                self.chassis.heading_rad,
                self.chassis.forward_speed_m_s(),
                dt,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vehicle::inputs::GearPosition;

    #[test]
    fn test_simulation_acceleration() {
        let mut sim = VehicleSimulation::new_standard_bus(11_500.0, 5.85);
        sim.inputs.set_parking_brake(false);
        sim.inputs.set_gear(GearPosition::Drive);
        sim.inputs.set_throttle(1.0);

        // Run for 2 seconds (120 steps at 60 Hz)
        for _ in 0..120 {
            sim.step(1.0 / 60.0);
        }

        // Bus should have moved forward and accelerated
        assert!(sim.chassis.forward_speed_km_h() > 10.0);
        assert!(sim.chassis.position.y > 0.0);
    }

    #[test]
    fn test_simulation_determinism() {
        let mut sim1 = VehicleSimulation::new_standard_bus(11_500.0, 5.85);
        sim1.inputs.set_parking_brake(false);
        sim1.inputs.set_gear(GearPosition::Drive);
        sim1.inputs.set_throttle(0.8);
        sim1.inputs.set_steering(0.3);

        let mut sim2 = sim1.clone();

        for _ in 0..60 {
            sim1.step(1.0 / 60.0);
            sim2.step(1.0 / 60.0);
        }

        // Must produce bit-for-bit identical results
        assert_eq!(sim1.chassis.position, sim2.chassis.position);
        assert_eq!(sim1.chassis.local_velocity, sim2.chassis.local_velocity);
        assert_eq!(sim1.chassis.heading_rad, sim2.chassis.heading_rad);
    }
}
