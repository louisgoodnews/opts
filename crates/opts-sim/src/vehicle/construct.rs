//! Constructing a vehicle simulation: the standard bus and the one from a manifest, and applying its spec.

use super::axle::AxleState;
use super::brakes::BrakeSystem;
use super::chassis::ChassisState;
use super::drivetrain::DrivetrainModel;
use super::inputs::VehicleInputs;
use super::spec::VehicleDynamicsSpec;
use super::suspension::SuspensionModel;
use super::{VehicleSimulation, ROAD_FRICTION};
use glam::Vec2;
use opts_core::formats::vehicles::bus::BusManifest;

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
            road_friction: ROAD_FRICTION,
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
            road_friction: ROAD_FRICTION,
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
}
