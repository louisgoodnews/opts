//! Vehicle controller component, player marker, input tuning and wheel visual components.
//!
//! The [`VehicleController`] wraps the authoritative simulation; the other types tag entities
//! and configure how the keyboard and the wheel meshes read it.

use bevy::prelude::*;
use opts_core::formats::vehicles::bus::BusManifest;
use opts_sim::vehicle::{
    axle::AxleState,
    inputs::{GearPosition, VehicleInputs},
    VehicleSimulation,
};

/// Component wrapping an authoritative [`VehicleSimulation`] state for an entity.
#[derive(Component, Debug, Clone)]
pub struct VehicleController {
    /// Authoritative simulation instance from `opts-sim`.
    pub sim: VehicleSimulation,
    /// The chassis pose before the last fixed step `(position, heading, elevation)`: the rendered
    /// pose interpolates from it to the current one by the fraction of the next step that has
    /// elapsed, so motion is smooth at any frame rate (the simulation steps at a fixed rate).
    pub render_prev: Option<(Vec2, f32, f32)>,
}

impl VehicleController {
    /// Constructs a new controller wrapping an existing [`VehicleSimulation`].
    #[must_use]
    pub fn new(sim: VehicleSimulation) -> Self {
        Self {
            sim,
            render_prev: None,
        }
    }

    /// Constructs a controller with a standard 12-meter transit bus configuration.
    #[must_use]
    pub fn standard_bus(mass_kg: f32, wheelbase_m: f32) -> Self {
        Self {
            sim: VehicleSimulation::new_standard_bus(mass_kg, wheelbase_m),
            render_prev: None,
        }
    }

    /// Constructs a controller from a parsed OMSI [`BusManifest`].
    #[must_use]
    pub fn from_manifest(manifest: &BusManifest) -> Self {
        Self {
            sim: VehicleSimulation::from_manifest(manifest),
            render_prev: None,
        }
    }

    /// Provides immutable access to active driver inputs.
    #[must_use]
    pub fn inputs(&self) -> &VehicleInputs {
        &self.sim.inputs
    }

    /// Provides mutable access to active driver inputs.
    pub fn inputs_mut(&mut self) -> &mut VehicleInputs {
        &mut self.sim.inputs
    }

    /// Returns the forward road speed in kilometers per hour.
    #[must_use]
    pub fn forward_speed_km_h(&self) -> f32 {
        self.sim.chassis.forward_speed_m_s() * 3.6
    }

    /// Returns the engine rotational speed in revolutions per minute.
    #[must_use]
    pub fn engine_rpm(&self) -> f32 {
        self.sim.drivetrain.current_rpm
    }

    /// Returns the current transmission gear selector position.
    #[must_use]
    pub fn gear(&self) -> GearPosition {
        self.sim.inputs.gear()
    }

    /// Returns slice of axle physical states.
    #[must_use]
    pub fn axles(&self) -> &[AxleState] {
        &self.sim.axles
    }
}

/// Marker component denoting the vehicle currently piloted by the local user.
#[derive(Component, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerVehicle;

/// Tuning parameters for smoothing keyboard driver inputs.
#[derive(Component, Clone, Copy, Debug)]
pub struct VehicleInputConfig {
    /// Acceleration rate of throttle input application in units/sec.
    pub throttle_rate: f32,
    /// Acceleration rate of service brake pedal application in units/sec.
    pub brake_rate: f32,
    /// Angular rate of steering lock application in units/sec.
    pub steering_rate: f32,
    /// Rate at which steering self-centers when no steering key is pressed.
    pub steering_return_rate: f32,
}

impl Default for VehicleInputConfig {
    fn default() -> Self {
        Self {
            throttle_rate: 2.5,
            brake_rate: 3.5,
            steering_rate: 2.0,
            steering_return_rate: 3.0,
        }
    }
}

/// Lateral side position of an individual road wheel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WheelSide {
    /// Left road wheel (+lateral in vehicle coordinates).
    Left,
    /// Right road wheel (-lateral in vehicle coordinates).
    Right,
}

/// Component attached to a child entity representing a 3D road wheel mesh.
#[derive(Component, Clone, Copy, Debug)]
pub struct WheelVisual {
    /// Zero-based index of the parent vehicle axle.
    pub axle_index: usize,
    /// Lateral axle side (Left or Right).
    pub side: WheelSide,
    /// Local rest translation vector relative to the vehicle root entity.
    pub rest_translation: Vec3,
}

impl WheelVisual {
    /// Creates a new wheel visual component.
    #[must_use]
    pub fn new(axle_index: usize, side: WheelSide, rest_translation: Vec3) -> Self {
        Self {
            axle_index,
            side,
            rest_translation,
        }
    }
}
