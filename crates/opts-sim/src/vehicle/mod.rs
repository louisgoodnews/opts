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
pub use self::consist::UnitLoad;
use self::consist::{driven_and_braked_load_n, UnitAxle, GRAVITY};
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

/// Newton metres at the wheels per unit of the scripts' `M_Wheel`. 1.0 until the calibration
/// experiment E1 (`docs/src/architecture/script-integration-report.md`) says otherwise ❓.
pub const M_WHEEL_UNIT: f32 = 1.0;
/// Newtons at the wheels per unit of the scripts' brake forces (`Brakeforce`,
/// `Axle_Brakeforce_*`). 1.0 until experiment E1 ❓.
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
            trailer_loads: Vec::new(),
            head_body: None,
            consist_bodies: Vec::new(),
            teleports: 0,
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
        let any_driven = self.axles.iter().any(|a| a.config.is_driven);
        let driven_wheels = self
            .axles
            .iter()
            .filter(|a| a.config.is_driven || !any_driven)
            .count()
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
    /// the hitch to the new unit's rearmost axle and `max_alpha_deg` the joint limit. `load` is
    /// the unit's mass and axles (`None`: the unit does not take part in the dynamics).
    pub fn attach_trailer(
        &mut self,
        hitch_local: DVec2,
        front_local: DVec2,
        axle_distance: f64,
        max_alpha_deg: f64,
        load: Option<UnitLoad>,
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
        self.consist_bodies.push(None);
        self.trailer_loads.push(load.unwrap_or(UnitLoad {
            mass_kg: 0.0,
            axles: Vec::new(),
        }));
    }

    /// The points to sample the ground at, one probe per unit: the chassis first, then each
    /// following unit (see [`ground`]).
    #[must_use]
    pub fn ground_probes(&self) -> Vec<GroundProbe> {
        let ends = self
            .axles
            .iter()
            .map(|a| a.config.longitudinal_m)
            .fold((f32::MIN, f32::MAX), |(hi, lo), y| (hi.max(y), lo.min(y)));
        let track = self
            .axles
            .iter()
            .map(|a| a.config.track_width_m)
            .fold(0.0_f32, f32::max);
        let track_m = if track > 0.5 { track } else { 2.0 };
        let (front_y, rear_y) = if ends.0 > f32::MIN {
            (ends.0, ends.1)
        } else {
            (3.0, -3.0)
        };
        let mut probes = vec![GroundProbe {
            origin: self.chassis.position,
            heading: self.chassis.heading_rad,
            rear_y,
            front_y,
            track_m,
        }];
        probes.extend(self.consist.iter().map(|u| GroundProbe {
            origin: Vec2::new(u.origin.x as f32, u.origin.y as f32),
            heading: u.heading as f32,
            rear_y: u.front_local.y as f32 - u.axle_distance as f32,
            front_y: u.front_local.y as f32,
            track_m,
        }));
        probes
    }

    /// Takes one fixed step of ground following: `samples[0]` is the chassis, `samples[k + 1]`
    /// consist unit `k` (`None`: no ground there, the unit keeps its state). A unit without a
    /// state yet starts on its goal. The filters run with `dt`, the fixed step, so the result
    /// depends on nothing else.
    pub fn apply_ground(&mut self, samples: &[Option<GroundSample>], dt: f32) {
        let settle = |body: &mut Option<UnitBody>, sample: &Option<GroundSample>| {
            let Some(sample) = sample else { return };
            let goal = sample.goal();
            match body {
                Some(b) => b.settle(goal, dt),
                None => *body = Some(goal),
            }
        };
        if let Some(s) = samples.first() {
            settle(&mut self.head_body, s);
        }
        for (k, body) in self.consist_bodies.iter_mut().enumerate() {
            if let Some(s) = samples.get(k + 1) {
                settle(body, s);
            }
        }
    }

    /// The fixed-step body of consist unit `k` (`None` before the first ground sample).
    #[must_use]
    pub fn consist_body(&self, k: usize) -> Option<UnitBody> {
        self.consist_bodies.get(k).copied().flatten()
    }

    /// `(β, γ)` in degrees of joint `k` (unit `k` against the unit ahead), the values behind
    /// `articulation_<k>_beta` and `_gamma` before the joint's `β` limits; `None` until both
    /// units have a body.
    #[must_use]
    pub fn joint_angles_deg(&self, k: usize) -> Option<(f32, f32)> {
        let leader = match k.checked_sub(1) {
            Some(j) => self.consist_body(j)?,
            None => self.head_body?,
        };
        let unit = self.consist_body(k)?;
        Some((
            joint_beta_deg(leader.pitch, unit.pitch),
            (unit.roll - leader.roll).to_degrees(),
        ))
    }

    /// Removes all following units.
    pub fn clear_consist(&mut self) {
        self.consist.clear();
        self.trailer_loads.clear();
        self.consist_bodies.clear();
        self.head_body = None;
    }

    /// Mass of the whole consist the longitudinal dynamics act on (kg): the chassis' own mass
    /// plus the following units' (design D2, `Σ mᵢ`).
    #[must_use]
    pub fn consist_mass_kg(&self) -> f32 {
        self.chassis.total_mass_kg + self.trailer_loads.iter().map(|u| u.mass_kg).sum::<f32>()
    }

    /// Mass of the head unit alone (kg): the chassis' own mass.
    #[must_use]
    pub fn head_mass_kg(&self) -> f32 {
        self.chassis.total_mass_kg
    }

    /// Mass standing on driven axles over all units (kg), `friction_loads_n().0 / g`.
    #[must_use]
    pub fn driven_mass_kg(&self) -> f32 {
        self.friction_loads_n().0 / GRAVITY
    }

    /// Declares the loads of the following units *without* poses (headless dynamics, tests):
    /// replaces `trailer_loads`. A consist with poses gets its loads through
    /// [`VehicleSimulation::attach_trailer`], which keeps `consist` and `trailer_loads` in step.
    pub fn set_trailers(&mut self, units: Vec<UnitLoad>) {
        self.trailer_loads = units;
    }

    /// Summed static normal force `(driven axles, braked axles)` over all units (N): what the
    /// tyres can transmit is `ROAD_FRICTION` times these. The chassis' axles carry its own mass
    /// (distributed by the suspension model), each following unit its own; every axle brakes.
    #[must_use]
    pub fn friction_loads_n(&self) -> (f32, f32) {
        let positions: Vec<f32> = self.axles.iter().map(|a| a.config.longitudinal_m).collect();
        let head = self
            .suspension
            .compute_static_loads(&positions, self.chassis.total_mass_kg);
        // A head without any axle marked driven counts as driven on all of them (the file does
        // not say; do not take away all traction).
        let head_has_driven = self.axles.iter().any(|a| a.config.is_driven);
        let mut axles: Vec<(UnitAxle, f32)> = self
            .axles
            .iter()
            .zip(head)
            .map(|(a, n)| {
                (
                    UnitAxle {
                        longitudinal_m: a.config.longitudinal_m,
                        is_driven: a.config.is_driven || !head_has_driven,
                        rated_n: a.config.max_force_n,
                    },
                    n,
                )
            })
            .collect();
        for unit in &self.trailer_loads {
            axles.extend(unit.axles.iter().copied().zip(unit.axle_loads_n()));
        }
        driven_and_braked_load_n(&axles)
    }

    /// Force the joint of consist unit `k` carries (N, report only; nothing is fed back): the
    /// mass behind it accelerated with the chassis plus its rolling resistance,
    /// `F = Σ_{j≥k} mⱼ (a + sign(v)·Crr·g)`. `None` for a missing unit.
    #[must_use]
    pub fn hitch_force_n(&self, k: usize) -> Option<f32> {
        let behind: f32 = self
            .trailer_loads
            .get(k..)
            .filter(|rest| !rest.is_empty())?
            .iter()
            .map(|u| u.mass_kg)
            .sum();
        let v = self.chassis.forward_speed_m_s();
        let direction = if v.abs() > 0.01 { v.signum() } else { 0.0 };
        let a = self.chassis.local_acceleration.y;
        Some(behind * (a + direction * drivetrain::ROLLING_RESISTANCE_COEFF * GRAVITY))
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
        // The next ground sample seeds the bodies on the new ground.
        self.head_body = None;
        self.consist_bodies.iter_mut().for_each(|b| *b = None);
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
        sim.attach_trailer(DVec2::new(0.0, -1.5), DVec2::new(0.0, 1.0), 7.0, 52.5, None);
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

    fn unit(mass_kg: f32, axles: &[(f32, bool)]) -> UnitLoad {
        UnitLoad {
            mass_kg,
            axles: axles
                .iter()
                .map(|&(y, is_driven)| UnitAxle {
                    longitudinal_m: y,
                    is_driven,
                    rated_n: consist::DEFAULT_RATED_AXLE_N,
                })
                .collect(),
        }
    }

    /// A bus of 11.5 t with a trailer of `trailer_kg` (none: solo), on a straight line.
    fn towing(trailer_kg: Option<f32>, trailer_driven: bool) -> VehicleSimulation {
        let mut sim = VehicleSimulation::new_standard_bus(11_500.0, 5.85);
        if let Some(m) = trailer_kg {
            let load = unit(m, &[(-4.0, trailer_driven)]);
            sim.attach_trailer(
                DVec2::new(0.0, -2.0),
                DVec2::new(0.0, 1.0),
                6.0,
                52.5,
                Some(load),
            );
        }
        sim.inputs.set_parking_brake(false);
        sim
    }

    /// Design V5 / acceptance 2: with the same wheel torque the acceleration scales with
    /// `1 / Σm` within 2 %. The traction part is the difference to a run without torque from
    /// the same state, so resistances cancel.
    #[test]
    fn script_torque_accelerates_the_whole_consist_mass() {
        let accel = |trailer: Option<f32>, torque: f32| {
            let mut sim = towing(trailer, false);
            sim.chassis.local_velocity.y = 5.0;
            sim.script_drive = Some(ScriptDrive {
                wheel_torque_nm: torque,
                brake_force_n: 0.0,
            });
            sim.step(1.0 / 64.0);
            sim.chassis.local_acceleration.y
        };
        let traction = |trailer| accel(trailer, 4_000.0) - accel(trailer, 0.0);
        let (solo, pair) = (traction(None), traction(Some(10_000.0)));
        let expected = solo * 11_500.0 / 21_500.0;
        assert!(
            (pair - expected).abs() < 0.02 * expected,
            "traction accel {pair} vs {expected}"
        );
        assert!((towing(Some(10_000.0), false).consist_mass_kg() - 21_500.0).abs() < 1e-3);
    }

    /// Plan step 4: stopping distance of head-only versus consist differs by the mass ratio
    /// (here: not at all, since brake force is limited by friction, `μ·g` per kilogram).
    #[test]
    fn brake_limit_follows_the_summed_axle_loads() {
        let stop = |sim: &mut VehicleSimulation| {
            sim.chassis.local_velocity.y = 14.0; // ~50 km/h
            sim.script_drive = Some(ScriptDrive {
                wheel_torque_nm: 0.0,
                brake_force_n: 1.0e9, // more than the tyres can carry
            });
            sim.inputs.set_service_brake(1.0);
            let mut s = 0.0_f32;
            for _ in 0..2000 {
                let before = sim.chassis.position.y;
                sim.step(1.0 / 64.0);
                s += sim.chassis.position.y - before;
                if sim.chassis.forward_speed_m_s() <= 0.0 {
                    break;
                }
            }
            s
        };
        let (solo, pair) = (
            stop(&mut towing(None, false)),
            stop(&mut towing(Some(10_000.0), false)),
        );
        // Same μ·g deceleration: the distance matches within 2 % (every axle brakes).
        assert!((solo - pair).abs() < 0.02 * solo, "{solo} vs {pair}");
        let ideal = 14.0_f32 * 14.0 / (2.0 * ROAD_FRICTION * GRAVITY);
        assert!(solo > ideal * 0.95, "{solo} vs ideal {ideal}");
    }

    /// Plan step 5: the drive cap follows the driven axles of all units.
    #[test]
    fn drive_limit_follows_the_driven_axles_of_all_units() {
        let head_only = towing(Some(10_000.0), false);
        let all_driven = towing(Some(10_000.0), true);
        let (d0, b0) = head_only.friction_loads_n();
        let (d1, b1) = all_driven.friction_loads_n();
        assert!((b0 - b1).abs() < 1.0, "braked load is the same");
        assert!((b0 - 21_500.0 * GRAVITY).abs() < 5.0);
        assert!((d1 - d0 - 10_000.0 * GRAVITY).abs() < 5.0, "{d0} -> {d1}");
        // The head's rear axle carries half its weight on this symmetric bus.
        assert!((d0 - 0.5 * 11_500.0 * GRAVITY).abs() < 5.0);
    }

    /// Plan step 6: on a straight run `F_h = m_last·a + resistances`.
    #[test]
    fn hitch_force_of_the_last_unit_is_mass_times_acceleration_plus_rolling() {
        let mut sim = towing(Some(10_000.0), false);
        sim.script_drive = Some(ScriptDrive {
            wheel_torque_nm: 6_000.0,
            brake_force_n: 0.0,
        });
        for _ in 0..64 {
            sim.step(1.0 / 64.0);
        }
        let a = sim.chassis.local_acceleration.y;
        let f = sim.hitch_force_n(0).unwrap();
        let want = 10_000.0 * (a + drivetrain::ROLLING_RESISTANCE_COEFF * GRAVITY);
        assert!(a > 0.1 && (f - want).abs() < 1.0, "{f} vs {want}");
        assert_eq!(sim.hitch_force_n(1), None);
    }

    #[test]
    fn loads_survive_serialisation() {
        let sim = towing(Some(9_000.0), true);
        let back: VehicleSimulation =
            bincode::deserialize(&bincode::serialize(&sim).unwrap()).unwrap();
        assert_eq!(sim, back);
    }

    /// Samples a plane `z = slope · y` under every probe.
    fn samples_on_slope(sim: &VehicleSimulation, slope: f32) -> Vec<Option<GroundSample>> {
        sim.ground_probes()
            .into_iter()
            .map(|probe| {
                let h = probe.points().map(|p| p.y * slope);
                Some(GroundSample::from_heights(probe, h))
            })
            .collect()
    }

    /// Acceptance 3 and the NG272 target in the harness: on flat ground pitch and roll stay 0;
    /// on a constant grade every unit takes the grade angle and `β` is 0, and a unit that stands
    /// on a different grade shows the difference of the two angles.
    #[test]
    fn fixed_step_ground_following_gives_zero_on_flat_and_the_grade_difference_on_a_step() {
        let mut sim = articulated();
        sim.inputs.set_throttle(0.0);
        sim.apply_ground(&samples_on_slope(&sim, 0.0), 1.0 / 64.0);
        let (beta, gamma) = sim.joint_angles_deg(0).unwrap();
        assert!(beta.abs() < 0.01 && gamma.abs() < 0.01);
        for _ in 0..200 {
            let s = samples_on_slope(&sim, 0.1);
            sim.apply_ground(&s, 1.0 / 64.0);
        }
        let grade_deg = 0.1_f32.atan().to_degrees();
        let (beta, _) = sim.joint_angles_deg(0).unwrap();
        // Chassis and unit both settle on the grade: β returns to 0 (the head was sampled on
        // flat ground first, so it has to catch up; after 3 s both agree).
        assert!(beta.abs() < 0.05, "{beta}");
        assert!((sim.consist_body(0).unwrap().pitch.to_degrees() - grade_deg).abs() < 0.05);
        // Unit on the grade, chassis still level: β is minus the grade angle (nose-up follower).
        let mut flat_head = articulated();
        let mut s = samples_on_slope(&flat_head, 0.0);
        s[1] = samples_on_slope(&flat_head, 0.1)[1];
        flat_head.apply_ground(&s, 1.0 / 64.0);
        let (beta, _) = flat_head.joint_angles_deg(0).unwrap();
        assert!((beta + grade_deg).abs() < 0.2, "{beta} vs {grade_deg}");
    }

    #[test]
    fn ground_state_is_reseeded_by_teleport_and_serialised() {
        let mut sim = articulated();
        sim.apply_ground(&samples_on_slope(&sim, 0.05), 1.0 / 64.0);
        let back: VehicleSimulation =
            bincode::deserialize(&bincode::serialize(&sim).unwrap()).unwrap();
        assert_eq!(sim, back);
        assert!(sim.joint_angles_deg(0).is_some());
        sim.teleport(Vec2::new(10.0, 10.0), 0.0);
        assert!(
            sim.joint_angles_deg(0).is_none(),
            "no body until the next sample"
        );
        // A missing sample keeps the state.
        sim.apply_ground(&samples_on_slope(&sim, 0.05), 1.0 / 64.0);
        let kept = sim.consist_body(0);
        sim.apply_ground(&[None, None], 1.0 / 64.0);
        assert_eq!(sim.consist_body(0), kept);
    }

    #[test]
    fn a_head_with_no_driven_axle_counts_as_driven_on_all() {
        let mut sim = VehicleSimulation::new_standard_bus(10_000.0, 5.0);
        sim.axles
            .iter_mut()
            .for_each(|a| a.config.is_driven = false);
        let (driven, braked) = sim.friction_loads_n();
        assert!((driven - braked).abs() < 1.0);
        sim.set_trailers(vec![UnitLoad::with_axles(5_000.0, 2, 2)]);
        assert!((sim.driven_mass_kg() - 15_000.0).abs() < 1.0);
    }
}

#[cfg(test)]
mod wheel_model_tests {
    use super::*;

    /// M-0097: under script drive the speed after 10 s at constant torque is the same, within
    /// 5 %, at 30, 60 and 120 steps per second.
    #[test]
    fn script_drive_does_not_depend_on_the_step_size() {
        let speed_after_10_s = |hz: f32| {
            let mut sim = VehicleSimulation::new_standard_bus(10_900.0, 5.85);
            sim.inputs.set_parking_brake(false);
            sim.script_drive = Some(ScriptDrive {
                wheel_torque_nm: 6000.0,
                brake_force_n: 0.0,
            });
            for _ in 0..(10.0 * hz) as usize {
                sim.step(1.0 / hz);
            }
            sim.chassis.forward_speed_m_s()
        };
        let (a, b, c) = (
            speed_after_10_s(30.0),
            speed_after_10_s(60.0),
            speed_after_10_s(120.0),
        );
        assert!(
            (a - b).abs() < 0.05 * b && (c - b).abs() < 0.05 * b,
            "{a} {b} {c}"
        );
        assert!(b > 8.0, "it drives: {b}");
    }

    /// The wheels of a vehicle under script drive turn with the road when nothing brakes.
    #[test]
    fn the_wheels_follow_the_road_speed_without_brakes() {
        let mut sim = VehicleSimulation::new_standard_bus(10_900.0, 5.85);
        sim.inputs.set_parking_brake(false);
        sim.chassis.local_velocity.y = 10.0;
        sim.script_drive = Some(ScriptDrive::default());
        for _ in 0..120 {
            sim.step(1.0 / 60.0);
        }
        let v = sim.chassis.forward_speed_m_s();
        let w = &sim.axles[0].left_wheel;
        assert!(
            (w.angular_velocity_rad_s * w.rolling_radius_m - v).abs() < 0.2,
            "{v}"
        );
    }

    /// The scripts' spring factor stiffens the body: under the same braking the pitch is smaller
    /// with a factor of 2, and a non-finite factor changes nothing.
    #[test]
    fn the_spring_factor_stiffens_the_suspension() {
        let pitch = |factor: f32| {
            let mut sim = VehicleSimulation::new_standard_bus(10_900.0, 5.85);
            sim.inputs.set_parking_brake(false);
            sim.chassis.local_velocity.y = 12.0;
            sim.script_drive = Some(ScriptDrive {
                wheel_torque_nm: 0.0,
                brake_force_n: 30_000.0,
            });
            sim.step(1.0 / 60.0); // builds the body
            for w in 0..sim.axles.len() * 2 {
                sim.set_spring_factor(w, factor);
                sim.set_spring_factor(w, f32::NAN);
            }
            for _ in 0..60 {
                sim.step(1.0 / 60.0);
            }
            sim.body.as_ref().expect("body").pitch_rad.abs()
        };
        let (soft, stiff) = (pitch(1.0), pitch(2.0));
        assert!(soft > 0.0 && stiff < 0.8 * soft, "{soft} vs {stiff}");
    }

    /// A vehicle the scripts' brakes hold on a slope does not roll backwards when the pedal
    /// is untouched (M-0097): the guard counts the script brake.
    #[test]
    fn a_script_brake_stops_the_roll_through_zero() {
        let mut sim = VehicleSimulation::new_standard_bus(10_900.0, 5.85);
        sim.inputs.set_parking_brake(false);
        sim.chassis.local_velocity.y = 1.0;
        sim.script_drive = Some(ScriptDrive {
            wheel_torque_nm: 0.0,
            brake_force_n: 60_000.0,
        });
        for _ in 0..600 {
            sim.step(1.0 / 60.0);
            assert!(
                sim.chassis.forward_speed_m_s() >= -0.001,
                "rolled backwards"
            );
        }
        assert!(sim.chassis.forward_speed_m_s().abs() < 0.01);
    }
}
