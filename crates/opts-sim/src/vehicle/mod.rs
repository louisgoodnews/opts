//! Vehicle physics, kinematics, and powertrain simulation.
//!
//! Provides the primary [`VehicleSimulation`] controller orchestrating multi-axle
//! Ackermann steering, longitudinal propulsion, braking systems, dynamic suspension
//! compliance, and optional multi-unit trailer articulation.

pub mod axle;
pub mod chassis;
pub mod drivetrain;
pub mod inputs;
pub mod suspension;
pub mod trailer;

use self::axle::AxleState;
use self::chassis::ChassisState;
use self::drivetrain::DrivetrainModel;
use self::inputs::VehicleInputs;
use self::suspension::SuspensionModel;
use self::trailer::TrailerKinematics;

use glam::{DVec2, Vec2};
use opts_core::formats::vehicles::bus::BusManifest;
use serde::{Deserialize, Serialize};

/// Drive torque and brake force the vehicle's own scripts hand to the physics (`M_Wheel`,
/// `Brakeforce`/`Axle_Brakeforce_*`); when set, they replace the built-in drivetrain's traction
/// and braking for the step (resistance, adhesion and the chassis stay with the physics).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct ScriptDrive {
    /// Torque at the driven wheels, summed over the axles (N·m; the unit of `M_Wheel`, see
    /// `docs/src/architecture/vehicle-scripts-route.md`, Q-VS-2). Negative drives backwards.
    pub wheel_torque_nm: f32,
    /// Total brake force at the wheels (N, never negative).
    pub brake_force_n: f32,
}

/// Friction coefficient between tyres and road that limits traction and braking of a road
/// vehicle (dry asphalt).
pub const ROAD_FRICTION: f32 = 0.85;

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
    /// The following units of an articulated bus or tram, front to back: unit `k` hangs on the
    /// rear coupling of unit `k - 1` (the chassis for `k = 0`). Stepped with the chassis in
    /// [`VehicleSimulation::step`] and serialised with it. Empty for a solitary vehicle.
    #[serde(default)]
    pub consist: Vec<TrailerKinematics>,
    /// Number of [`VehicleSimulation::teleport`] calls so far (wrapping). Readers that keep
    /// per-step state of their own compare it with the last value they saw to learn that the
    /// consist was re-seeded.
    #[serde(default)]
    pub teleports: u32,
    /// Active driver control inputs.
    pub inputs: VehicleInputs,
    /// Effective wheelbase between front steering axle and rear reference axle in meters.
    pub wheelbase_m: f32,
    /// Torque and brake force from the vehicle's scripts for the next steps (`None`: the
    /// built-in drivetrain decides).
    #[serde(default)]
    pub script_drive: Option<ScriptDrive>,
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
            consist: Vec::new(),
            teleports: 0,
            inputs: VehicleInputs::new(),
            wheelbase_m: wheelbase,
            script_drive: None,
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

        Self {
            chassis: ChassisState::new(mass_kg, Vec2::ZERO, 0.0),
            axles,
            drivetrain: DrivetrainModel::new(),
            suspension: SuspensionModel::new(1.15, 0.0),
            consist: Vec::new(),
            teleports: 0,
            inputs: VehicleInputs::new(),
            wheelbase_m: wheelbase,
            script_drive: None,
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
        let mut forces = self.drivetrain.step(
            &self.inputs,
            speed,
            first_wheel_radius,
            self.chassis.total_mass_kg,
        );
        if let Some(d) = self.script_drive {
            // The scripts' torque and brakes, limited by what the tyres can carry.
            let limit = ROAD_FRICTION * self.chassis.total_mass_kg * 9.81;
            let driven_radius = self
                .axles
                .iter()
                .find(|a| a.config.is_driven)
                .map_or(first_wheel_radius, |a| a.config.wheel_diameter_m * 0.5)
                .max(0.1);
            forces.tractive_force_n = (d.wheel_torque_nm / driven_radius).clamp(-limit, limit);
            forces.braking_force_n = d.brake_force_n.clamp(0.0, limit);
        }

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

        // 6. Consist kinematics: every unit follows the one ahead, starting with the chassis.
        let mut leader = self.chassis_pose();
        for unit in &mut self.consist {
            let st = unit.step(leader.0, leader.1, f64::from(dt));
            leader = (st.origin, st.heading);
        }
    }

    /// Position and heading (compass radians) of the chassis in the precision of the consist.
    fn chassis_pose(&self) -> (DVec2, f64) {
        let p = self.chassis.position;
        (
            DVec2::new(f64::from(p.x), f64::from(p.y)),
            f64::from(self.chassis.heading_rad),
        )
    }

    /// Pose `(origin, heading)` of the unit ahead of consist unit `k`: the chassis for `k = 0`.
    fn leader_pose_of(&self, k: usize) -> (DVec2, f64) {
        match k.checked_sub(1).and_then(|j| self.consist.get(j)) {
            Some(ahead) => (ahead.origin, ahead.heading),
            None => self.chassis_pose(),
        }
    }

    /// Adds a following unit behind the last one (or behind the chassis), hanging straight.
    ///
    /// `hitch_local` is the rear coupling of the unit ahead and `front_local` the front coupling
    /// of the new unit, both `(x, y)` in their own frames; `axle_distance` is the distance from
    /// the hitch to the new unit's rearmost axle and `max_alpha_deg` the joint limit.
    pub fn attach_trailer(
        &mut self,
        hitch_local: DVec2,
        front_local: DVec2,
        axle_distance: f64,
        max_alpha_deg: f64,
    ) {
        let (pos, heading) = self.leader_pose_of(self.consist.len());
        self.consist.push(TrailerKinematics::new(
            hitch_local,
            front_local,
            axle_distance,
            max_alpha_deg,
            pos,
            heading,
        ));
    }

    /// Yaw angle of consist unit `k` against the unit ahead in radians (positive = turned
    /// clockwise), the value published as `articulation_<k>_alpha`; `None` for a missing unit.
    #[must_use]
    pub fn consist_alpha_rad(&self, k: usize) -> Option<f64> {
        let unit = self.consist.get(k)?;
        Some(unit.alpha_against(self.leader_pose_of(k).1))
    }

    /// Re-seeds every following unit straight behind its leader, without velocity.
    pub fn reseed_consist(&mut self) {
        for k in 0..self.consist.len() {
            let (pos, heading) = self.leader_pose_of(k);
            self.consist[k].reseed(pos, heading);
        }
    }

    /// Places the chassis at `position` heading `heading_rad` (compass radians) and re-seeds the
    /// consist behind it: the explicit response to a spawn, respawn or reset. Velocities of the
    /// chassis are left to the caller.
    pub fn teleport(&mut self, position: Vec2, heading_rad: f32) {
        self.chassis.position = position;
        self.chassis.heading_rad = heading_rad;
        self.reseed_consist();
        self.teleports = self.teleports.wrapping_add(1);
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

    /// A bus with one unit behind it: hitch 1.5 m behind the chassis origin, front coupling
    /// 1 m ahead of the unit's origin, rear axle 6 m behind that.
    fn articulated() -> VehicleSimulation {
        let mut sim = VehicleSimulation::new_standard_bus(11_500.0, 5.85);
        sim.attach_trailer(DVec2::new(0.0, -1.5), DVec2::new(0.0, 1.0), 7.0, 52.5);
        sim.inputs.set_parking_brake(false);
        sim.inputs.set_gear(GearPosition::Drive);
        sim.inputs.set_throttle(0.4);
        sim.inputs.set_steering(0.5);
        sim
    }

    #[test]
    fn the_consist_is_stepped_with_the_chassis_and_stays_hitched() {
        let mut sim = articulated();
        for _ in 0..600 {
            sim.step(1.0 / 64.0);
        }
        let alpha = sim.consist_alpha_rad(0).unwrap();
        assert!(alpha.abs() > 0.05 && alpha.abs() <= 52.5_f64.to_radians());
        let unit = &sim.consist[0];
        let (pos, heading) = sim.chassis_pose();
        let hitch = unit.hitch(pos, heading);
        let front = unit.origin
            + DVec2::new(unit.heading.cos(), -unit.heading.sin()) * unit.front_local.x
            + DVec2::new(unit.heading.sin(), unit.heading.cos()) * unit.front_local.y;
        assert!((front - hitch).length() < 1e-6);
        assert_eq!(sim.consist_alpha_rad(1), None);
    }

    /// Design V4: a consist driven into a circle, serialised, restored and driven on continues
    /// exactly like the unbroken run (the acceptance asks for 1 mm and 0.01 degrees).
    #[test]
    fn a_serialised_consist_resumes_like_the_unbroken_run() {
        let mut a = articulated();
        for _ in 0..400 {
            a.step(1.0 / 64.0);
        }
        let bytes = bincode::serialize(&a).unwrap();
        let mut b: VehicleSimulation = bincode::deserialize(&bytes).unwrap();
        assert_eq!(a, b, "the restored state equals the saved one");
        for _ in 0..200 {
            a.step(1.0 / 64.0);
            b.step(1.0 / 64.0);
        }
        assert_eq!(a, b);
        let d = (a.consist[0].origin - b.consist[0].origin).length();
        let dalpha = (a.consist_alpha_rad(0).unwrap() - b.consist_alpha_rad(0).unwrap()).abs();
        assert!(d < 1e-3 && dalpha.to_degrees() < 0.01);
        assert!(
            a.consist_alpha_rad(0).unwrap().abs() > 0.05,
            "it is in a curve"
        );
    }

    #[test]
    fn a_state_saved_without_a_consist_still_loads() {
        // `consist` and `teleports` are `serde(default)`; a vehicle of that shape round-trips.
        let solo = VehicleSimulation::default();
        let back: VehicleSimulation =
            bincode::deserialize(&bincode::serialize(&solo).unwrap()).unwrap();
        assert!(back.consist.is_empty());
        assert_eq!(solo, back);
    }

    #[test]
    fn teleport_hangs_the_consist_straight_with_no_velocity() {
        let mut sim = articulated();
        for _ in 0..400 {
            sim.step(1.0 / 64.0);
        }
        assert!(sim.consist_alpha_rad(0).unwrap().abs() > 0.05);
        sim.teleport(Vec2::new(500.0, -300.0), 1.2);
        assert_eq!(sim.teleports, 1);
        assert!(sim.consist_alpha_rad(0).unwrap().abs() < 1e-12);
        // Standing still after the jump: the unit does not move.
        sim.inputs.set_throttle(0.0);
        sim.inputs.set_steering(0.0);
        sim.chassis.local_velocity = Vec2::ZERO;
        sim.chassis.yaw_rate_rad_s = 0.0;
        let before = sim.consist[0].origin;
        sim.step(1.0 / 64.0);
        assert!((sim.consist[0].origin - before).length() < 1e-6);
        assert!(sim.consist_alpha_rad(0).unwrap().abs() < 1e-6);
    }
}
