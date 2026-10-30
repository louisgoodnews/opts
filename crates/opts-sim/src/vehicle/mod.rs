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
    /// Mass of the people on board each following unit (kg; same order as
    /// [`VehicleSimulation::trailer_loads`], missing entries are 0): the per-unit load of a
    /// consist (M-0124). Saves from before it have none and load as empty units.
    #[serde(default)]
    pub unit_passenger_mass_kg: Vec<f32>,
}

impl Default for VehicleSimulation {
    fn default() -> Self {
        Self::new_standard_bus(11_500.0, 5.85)
    }
}

mod consist_ops;
mod ground_ops;
mod step;

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
            unit_passenger_mass_kg: Vec::new(),
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
            unit_passenger_mass_kg: Vec::new(),
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

    /// Position and heading (compass radians) of the chassis in the precision of the consist.
    pub(super) fn chassis_pose(&self) -> (DVec2, f64) {
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
