//! Vehicle physics, kinematics, and powertrain simulation.
//!
//! Provides the primary [`VehicleSimulation`] controller orchestrating multi-axle
//! Ackermann steering, longitudinal propulsion, braking systems, dynamic suspension
//! compliance, and optional multi-unit trailer articulation.

pub mod axle;
pub mod brakes;
pub mod chassis;
pub mod consist;
pub mod drivetrain;
pub mod ground;
pub mod inputs;
pub mod spec;
pub mod suspension;
pub mod trailer;
pub mod wheels;

use self::axle::AxleState;
use self::brakes::{BrakeSystem, BRAKE_FACTOR};
use self::chassis::ChassisState;
use self::consist::{driven_and_braked_load_n, UnitAxle, GRAVITY};
pub use self::consist::{dynamic_mass_kg, UnitLoad};
use self::drivetrain::DrivetrainModel;
use self::ground::{joint_beta_deg, GroundProbe, GroundSample, UnitBody};
use self::inputs::VehicleInputs;
use self::spec::VehicleDynamicsSpec;
use self::suspension::{BodySuspension, SuspensionModel};
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

/// Newton metres at the wheels per unit of the scripts' `M_Wheel`: 1.0, the drive script forms it
/// as engine torque times the total ratio and checks its power with `M_Wheel · n_Wheel · π / 30000`
/// kW (`docs/src/architecture/script-integration-report.md`, the units read from the scripts).
pub const M_WHEEL_UNIT: f32 = 1.0;
/// Newtons at the wheels per unit of the scripts' brake forces (`Brakeforce`,
/// `Axle_Brakeforce_*`): 1.0, the brake script's maximum (`max_bremskraft`, 90,000) is about the
/// friction limit of the bus in newtons.
pub const BRAKE_FORCE_UNIT: f32 = 1.0;

/// Friction coefficient between tyres and road that limits traction and braking of a road
/// vehicle (dry asphalt).
pub const ROAD_FRICTION: f32 = 0.85;

/// Speed below which a vehicle counts as standing (m/s).
const STANDSTILL_M_S: f32 = 0.01;

/// The net force along the vehicle (N, forward positive). Moving, the brakes and the resistances
/// oppose the motion; standing, they hold: the vehicle stays put while the driving force
/// (`tractive` plus gravity along the road, `grade`) does not exceed what the brakes and the
/// rolling resistance withstand, and otherwise moves under the excess.
fn longitudinal_force(
    speed: f32,
    tractive: f32,
    grade: f32,
    brake: f32,
    drag: f32,
    rolling: f32,
) -> f32 {
    if speed.abs() > STANDSTILL_M_S {
        let against = if speed > 0.0 { -1.0 } else { 1.0 };
        return tractive + grade + against * (brake + drag + rolling);
    }
    let driving = tractive + grade;
    let hold = brake + rolling;
    if driving.abs() <= hold {
        0.0
    } else {
        driving - driving.signum() * hold
    }
}

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
    /// Mass and axles of each unit of [`VehicleSimulation::consist`] (same order; the chassis'
    /// own mass and axles are `chassis.total_mass_kg` and `axles`). A unit attached without a
    /// load counts as massless.
    #[serde(default)]
    pub trailer_loads: Vec<UnitLoad>,
    /// Height, pitch and roll of the chassis as the fixed step sees them (`None` until the first
    /// ground sample): the leader value of the first joint's `β`/`γ`. The visible body motion of
    /// the chassis is separate (`opts-game` `follow_ground`).
    #[serde(default)]
    pub head_body: Option<UnitBody>,
    /// The same for each following unit, in the order of [`VehicleSimulation::consist`].
    #[serde(default)]
    pub consist_bodies: Vec<Option<UnitBody>>,
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
    /// The wheels carry their own speed from the last step (the wheel model ran); otherwise the
    /// next wheel step starts them at the rolling speed of the vehicle.
    #[serde(default)]
    pub wheels_spinning: bool,
    /// What the vehicle file states about mass, centre of gravity, inertia and rolling
    /// resistance, and where each value comes from (M-0073).
    #[serde(default)]
    pub spec: VehicleDynamicsSpec,
    /// The body on its wheels: per-wheel spring and damper, heave, pitch and roll (M-0073).
    /// `None` until the first step builds it from the axles.
    #[serde(default)]
    pub body: Option<BodySuspension>,
    /// The service brake's pressure and per-axle forces (M-0073).
    #[serde(default)]
    pub brakes: BrakeSystem,
    /// Height of the ground under each wheel against the plane the chassis follows (m; axle by
    /// axle, left then right): the host fills it from the terrain, missing entries are 0.
    #[serde(default)]
    pub wheel_ground_m: Vec<f32>,
    /// Mass of the people on board (kg), added to the mass the longitudinal dynamics move
    /// (M-0124); the host sets it, 0 for an empty vehicle. The axle loads and the suspension
    /// still use the curb mass.
    #[serde(default)]
    pub passenger_mass_kg: f32,
}

impl Default for VehicleSimulation {
    fn default() -> Self {
        Self::new_standard_bus(11_500.0, 5.85)
    }
}

mod consist_ops;

mod ground_ops;

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
            trailer_loads: Vec::new(),
            head_body: None,
            consist_bodies: Vec::new(),
            teleports: 0,
            passenger_mass_kg: 0.0,
            inputs: VehicleInputs::new(),
            wheelbase_m: wheelbase,
            script_drive: None,
            wheels_spinning: false,
            spec: VehicleDynamicsSpec::default(),
            body: None,
            brakes: BrakeSystem::default(),
            wheel_ground_m: Vec::new(),
        }
    }

    /// Constructs a full vehicle simulation from a parsed OMSI [`BusManifest`].
    #[must_use]
    pub fn from_manifest(manifest: &BusManifest) -> Self {
        let spec = VehicleDynamicsSpec::from_manifest(manifest);
        let mass_kg = spec.mass_kg.value;

        let mut axles = Vec::new();

        for (idx, manifest_axle) in manifest.axles.iter().enumerate() {
            // By convention in OMSI manifests, the first axle (index 0) is the steer axle
            let is_steered = idx == 0;
            axles.push(AxleState::from_manifest(manifest_axle, is_steered));
        }

        // If manifest had no axles, supply default front and rear axles
        if axles.is_empty() {
            let mut sim = Self::new_standard_bus(mass_kg, 5.85);
            sim.apply_spec(spec);
            return sim;
        }

        let y_front = axles.first().map_or(3.0, |a| a.config.longitudinal_m);
        let y_rear = axles.last().map_or(-3.0, |a| a.config.longitudinal_m);
        let wheelbase = (y_front - y_rear).abs().max(1.0);

        let mut sim = Self {
            chassis: ChassisState::new(mass_kg, Vec2::ZERO, 0.0),
            axles,
            drivetrain: DrivetrainModel::new(),
            suspension: SuspensionModel::new(1.15, 0.0),
            consist: Vec::new(),
            trailer_loads: Vec::new(),
            head_body: None,
            consist_bodies: Vec::new(),
            teleports: 0,
            passenger_mass_kg: 0.0,
            inputs: VehicleInputs::new(),
            wheelbase_m: wheelbase,
            script_drive: None,
            wheels_spinning: false,
            spec: VehicleDynamicsSpec::default(),
            body: None,
            brakes: BrakeSystem::default(),
            wheel_ground_m: Vec::new(),
        };
        sim.apply_spec(spec);
        sim
    }

    /// Makes the vehicle behave as its spec says: the centre of gravity height of the
    /// suspension model, the yaw inertia of the chassis and the rolling resistance of the
    /// drivetrain.
    pub fn apply_spec(&mut self, spec: VehicleDynamicsSpec) {
        self.suspension =
            SuspensionModel::new(spec.cg_height_m.value, self.suspension.cg_longitudinal_m);
        self.chassis.yaw_inertia_kg_m2 = spec.yaw_inertia_kg_m2.value;
        self.drivetrain.rolling_coefficient = spec.rolling_coefficient.value;
        self.spec = spec;
        // Rebuilt from the new inertia and centre of gravity on the next step.
        self.body = None;
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

        // The wheel loads the tyres need exist from the first step on.
        if self.body.is_none() {
            self.rebuild_body();
        }

        // 2. Drivetrain & braking forces
        let moving_mass_kg = self.consist_mass_kg();
        let mut forces =
            self.drivetrain
                .step(&self.inputs, speed, first_wheel_radius, moving_mass_kg);
        let mut wheel_forces = false;
        let was_spinning = std::mem::replace(&mut self.wheels_spinning, false);
        if let Some(d) = self.script_drive {
            if speed.abs() > STANDSTILL_M_S {
                // Moving: the scripts' torque and brakes act on the spinning wheels; the tyres
                // turn them into the longitudinal force (never above the friction limit).
                if !was_spinning {
                    for axle in &mut self.axles {
                        for w in [&mut axle.left_wheel, &mut axle.right_wheel] {
                            w.angular_velocity_rad_s = speed / w.rolling_radius_m.max(0.1);
                        }
                    }
                }
                forces.tractive_force_n = self.step_wheels(d, speed, dt);
                forces.braking_force_n = 0.0;
                wheel_forces = true;
                self.wheels_spinning = true;
            } else {
                // Standing: static hold against the scripts' torque, limited by the tyres.
                let (driven_n, braked_n) = self.friction_loads_n();
                let (drive_limit, brake_limit) =
                    (ROAD_FRICTION * driven_n, ROAD_FRICTION * braked_n);
                let driven_radius = self
                    .axles
                    .iter()
                    .find(|a| a.config.is_driven)
                    .map_or(first_wheel_radius, |a| a.config.wheel_diameter_m * 0.5)
                    .max(0.1);
                forces.tractive_force_n =
                    (d.wheel_torque_nm / driven_radius).clamp(-drive_limit, drive_limit);
                forces.braking_force_n = d.brake_force_n.clamp(0.0, brake_limit);
                for axle in &mut self.axles {
                    for w in [&mut axle.left_wheel, &mut axle.right_wheel] {
                        w.angular_velocity_rad_s = speed / w.rolling_radius_m.max(0.1);
                    }
                }
            }
        }

        // The service brake: built-in drivetrain only (a script drive brings its own force).
        if self.script_drive.is_none() {
            let (caps, limits) = self.brake_axles();
            self.brakes
                .size_for(caps.iter().sum::<f32>() / BRAKE_FACTOR);
            forces.braking_force_n +=
                self.brakes
                    .step(self.inputs.service_brake(), &caps, &limits, dt);
        } else {
            self.brakes.pressure = 0.0;
        }

        // Gravity along the road: the chassis pitch of the last ground sample, nose up positive.
        let grade_n = -self.head_body.map_or(0.0, |b| b.pitch.sin()) * moving_mass_kg * GRAVITY;
        let net_longitudinal_force = longitudinal_force(
            speed,
            forces.tractive_force_n,
            grade_n,
            forces.braking_force_n,
            forces.drag_force_n,
            forces.rolling_resistance_n,
        );

        // 3. Kinematic turning yaw rate: omega = (v / L) * tan(delta)
        let yaw_rate = (speed / self.wheelbase_m) * central_steer_rad.tan();
        let yaw_torque =
            (yaw_rate - self.chassis.yaw_rate_rad_s) * self.chassis.yaw_inertia_kg_m2 * 10.0;

        // 4. Integrate chassis dynamics
        self.chassis.integrate_with_mass(
            net_longitudinal_force,
            yaw_torque,
            // Under script drive the scripts' brakes count, not the pedal alone.
            self.inputs.is_braking() || self.script_drive.is_some_and(|d| d.brake_force_n > 0.0),
            dt,
            moving_mass_kg,
        );

        // 5. Axle rotation and the body on its wheels: heave, pitch, roll and the load on every
        // wheel from the accelerations and the ground under the wheels.
        if self.body.is_none() {
            self.rebuild_body();
        }
        let a_lat = speed * self.chassis.yaw_rate_rad_s;
        let a_long = self.chassis.local_acceleration.y;
        self.sync_payload();
        if let Some(body) = self.body.as_mut() {
            body.step(dt, a_long, a_lat, &self.wheel_ground_m);
            for (i, axle) in self.axles.iter_mut().enumerate() {
                if let Some(w) = body.wheels.get(i * 2..i * 2 + 2) {
                    axle.left_wheel.vertical_load_n = w[0].load_n;
                    axle.right_wheel.vertical_load_n = w[1].load_n;
                    axle.suspension_deflection_m = 0.5 * (w[0].compression_m + w[1].compression_m);
                    axle.suspension_velocity_m_s =
                        0.5 * (w[0].compression_rate_m_s + w[1].compression_rate_m_s);
                }
            }
        }
        if !wheel_forces {
            for axle in &mut self.axles {
                axle.update_rotation(self.chassis.forward_speed_m_s(), dt);
            }
        }

        // 6. Consist kinematics: every unit follows the one ahead, starting with the chassis.
        let mut leader = self.chassis_pose();
        for unit in &mut self.consist {
            let st = unit.step(leader.0, leader.1, f64::from(dt));
            leader = (st.origin, st.heading);
        }
    }

    /// Advances the wheels of the head's axles under the scripts' torque and brake force and
    /// returns the sum of their longitudinal tyre forces (N). The torque goes to the driven
    /// wheels (all of them when no axle is marked driven) in equal parts, the brake force to the
    /// head's axles in proportion to their rated force.
    fn step_wheels(&mut self, d: ScriptDrive, speed: f32, dt: f32) -> f32 {
        let unit_driven = self
            .trailer_loads
            .iter()
            .flat_map(|u| u.axles.iter())
            .filter(|a| a.is_driven)
            .count();
        let any_driven = self.axles.iter().any(|a| a.config.is_driven) || unit_driven > 0;
        let driven_wheels = (self
            .axles
            .iter()
            .filter(|a| a.config.is_driven || !any_driven)
            .count()
            + unit_driven)
            * 2;
        let torque_each = d.wheel_torque_nm / driven_wheels.max(1) as f32;
        let rated_head: f32 = self.axles.iter().map(|a| a.config.max_force_n).sum();
        let rated_all: f32 = rated_head
            + self
                .trailer_loads
                .iter()
                .flat_map(|u| u.axles.iter())
                .map(|a| a.rated_n)
                .sum::<f32>();
        let head_brake_n = if rated_all > 0.0 {
            d.brake_force_n * rated_head / rated_all
        } else {
            0.0
        };
        let mut total = 0.0;
        let axle_count = self.axles.len().max(1) as f32;
        for axle in &mut self.axles {
            let drive = if axle.config.is_driven || !any_driven {
                torque_each
            } else {
                0.0
            };
            let share = if rated_head > 0.0 {
                axle.config.max_force_n / rated_head
            } else {
                1.0 / axle_count
            };
            for w in [&mut axle.left_wheel, &mut axle.right_wheel] {
                let r = w.rolling_radius_m.max(0.1);
                let brake_nm = head_brake_n * share * 0.5 * r;
                let load = w.vertical_load_n;
                let st = wheels::step_wheel(w, drive, brake_nm, load, speed, ROAD_FRICTION, dt);
                total += st.force_n;
            }
        }
        // The driven axles of the following units (the driven axle of an articulated bus hangs
        // in its second unit): the tyre carries the torque as far as its load allows.
        let r_unit = self
            .axles
            .first()
            .map_or(0.49, |a| a.left_wheel.rolling_radius_m)
            .max(0.1);
        for unit in &self.trailer_loads {
            for (axle, load) in unit.axles.iter().zip(unit.axle_loads_n()) {
                if axle.is_driven {
                    let limit = ROAD_FRICTION * load;
                    total += (2.0 * torque_each / r_unit).clamp(-limit, limit);
                }
            }
        }
        // The following units: their brake share, as far as their tyres carry it.
        let trailer_brake_n = d.brake_force_n - head_brake_n;
        if trailer_brake_n > 0.0 {
            let loads: f32 = self
                .trailer_loads
                .iter()
                .flat_map(|u| u.axle_loads_n())
                .sum();
            total -= speed.signum() * trailer_brake_n.min(ROAD_FRICTION * loads);
        }
        total
    }

    /// Sets the factor on the spring of wheel `wheel` (axle `n`: left `2n`, right `2n + 1`) from
    /// the scripts' `Axle_Springfactor`; non-finite values and wheels that do not exist are
    /// ignored, the factor is held to [`suspension::SPRING_FACTOR_RANGE`].
    pub fn set_spring_factor(&mut self, wheel: usize, factor: f32) {
        if !factor.is_finite() {
            return;
        }
        let (lo, hi) = suspension::SPRING_FACTOR_RANGE;
        if let Some(w) = self.body.as_mut().and_then(|b| b.wheels.get_mut(wheel)) {
            w.spring_factor = factor.clamp(lo, hi);
        }
    }

    /// Builds the body-on-wheels model from the axles, the mass and the spec (at rest, on the
    /// static axle loads).
    pub fn rebuild_body(&mut self) {
        let positions: Vec<f32> = self.axles.iter().map(|a| a.config.longitudinal_m).collect();
        let loads = self
            .suspension
            .compute_static_loads(&positions, self.chassis.total_mass_kg);
        let configs: Vec<_> = self.axles.iter().map(|a| a.config.clone()).collect();
        self.body = Some(BodySuspension::new(
            &configs,
            &loads,
            self.chassis.total_mass_kg,
            self.spec.pitch_inertia_kg_m2.value,
            self.spec.roll_inertia_kg_m2.value,
            self.suspension.cg_height_m,
        ));
        for (axle, load) in self.axles.iter_mut().zip(&loads) {
            axle.left_wheel.vertical_load_n = load * 0.5;
            axle.right_wheel.vertical_load_n = load * 0.5;
        }
    }

    /// Brake force at full pressure and what the tyres carry now, per axle over all units (the
    /// head's axles first, then each following unit's): the inputs of the [`BrakeSystem`].
    fn brake_axles(&self) -> (Vec<f32>, Vec<f32>) {
        let (mut caps, mut limits) = (Vec::new(), Vec::new());
        for (i, a) in self.axles.iter().enumerate() {
            caps.push(BRAKE_FACTOR * a.config.max_force_n);
            let load = self.body.as_ref().map_or(
                a.left_wheel.vertical_load_n + a.right_wheel.vertical_load_n,
                |b| b.axle_load_n(i),
            );
            limits.push(ROAD_FRICTION * load);
        }
        for unit in &self.trailer_loads {
            for (axle, load) in unit.axles.iter().zip(unit.axle_loads_n()) {
                caps.push(BRAKE_FACTOR * axle.rated_n);
                limits.push(ROAD_FRICTION * load);
            }
        }
        (caps, limits)
    }

    /// Position and heading (compass radians) of the chassis in the precision of the consist.
    fn chassis_pose(&self) -> (DVec2, f64) {
        let p = self.chassis.position;
        (
            DVec2::new(f64::from(p.x), f64::from(p.y)),
            f64::from(self.chassis.heading_rad),
        )
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
mod tests;

#[cfg(test)]
mod wheel_model_tests;
