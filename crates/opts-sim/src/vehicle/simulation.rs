//! The vehicle simulation state: chassis, axles, drivetrain, suspension, consist and inputs.

use super::axle::AxleState;
use super::aids::DriverAids;
use super::brakes::BrakeSystem;
use super::chassis::ChassisState;
use super::consist::UnitLoad;
use super::drivetrain::DrivetrainModel;
use super::fuel::FuelState;
use super::ground::UnitBody;
use super::inputs::VehicleInputs;
use super::spec::VehicleDynamicsSpec;
use super::surface::VehicleSurface;
use super::suspension::{BodySuspension, SuspensionModel};
use super::trailer::TrailerKinematics;
use super::ScriptDrive;
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
    /// The couplings' heights `[hitch_z, front_z]` of each following unit (see
    /// [`VehicleSimulation::set_consist_couplings_z`]), in the order of [`VehicleSimulation::consist`].
    #[serde(default)]
    pub consist_couplings: Vec<[f64; 2]>,
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
    /// The ABS and ASR driver aids (ISS-043): their switches and the shares they let through.
    #[serde(default)]
    pub driver_aids: DriverAids,
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
    /// Friction coefficient between the tyres and the road now (M-0117): the dry road by
    /// default, set by the host from [`crate::surface_grip`] as the weather changes. It limits the
    /// tyre forces of traction and braking. Saves from before it load with the dry value.
    #[serde(default = "default_road_friction")]
    pub road_friction: f32,
    /// Fuel or battery charge (M-0123); `None` for a vehicle without the model, which then
    /// drives as before. Saves from before it load with `None`.
    #[serde(default)]
    pub fuel: Option<FuelState>,
    /// Host surface state: dirt (`Dirt_Norm`), wetness and the last wash (M-0120, M-0121). Saves
    /// from before it load clean.
    #[serde(default)]
    pub surface: VehicleSurface,
}

fn default_road_friction() -> f32 {
    super::ROAD_FRICTION
}

impl Default for VehicleSimulation {
    fn default() -> Self {
        Self::new_standard_bus(11_500.0, 5.85)
    }
}
