//! The drivetrain model: its state, the forces it outputs and its default parameters.

use super::specs::{AerodynamicSpec, EngineSpec, TransmissionSpec};
use serde::{Deserialize, Serialize};

/// Dynamic output forces delivered by the powertrain and brake systems per tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DrivetrainForces {
    /// Forward tractive propulsion force in Newtons.
    pub tractive_force_n: f32,
    /// Total retarding brake force (service + retarder + parking) in Newtons.
    pub braking_force_n: f32,
    /// Aerodynamic retarding drag force in Newtons.
    pub drag_force_n: f32,
    /// Rolling resistance retarding force in Newtons.
    pub rolling_resistance_n: f32,
    /// Engine rotational speed in RPM.
    pub engine_rpm: f32,
    /// Active gear index (0 = Neutral, 1..=N = Forward, 255 = Reverse).
    pub active_gear: u8,
}

/// Rolling resistance coefficient of the road vehicle (force = coefficient · weight).
pub const ROLLING_RESISTANCE_COEFF: f32 = 0.012;

/// High-level powertrain and brake simulator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DrivetrainModel {
    /// Engine specification.
    pub engine: EngineSpec,
    /// Transmission specification.
    pub transmission: TransmissionSpec,
    /// Aerodynamic specification.
    pub aero: AerodynamicSpec,
    /// Current engine speed in RPM.
    pub current_rpm: f32,
    /// Current engaged forward gear index (1-indexed).
    pub current_gear: usize,
    /// Rolling resistance as force over weight: the vehicle file's `[rollwiderstand]` over its
    /// weight when it states one (`VehicleDynamicsSpec`), else [`ROLLING_RESISTANCE_COEFF`].
    #[serde(default = "default_rolling_coefficient")]
    pub rolling_coefficient: f32,
}

fn default_rolling_coefficient() -> f32 {
    ROLLING_RESISTANCE_COEFF
}

impl Default for DrivetrainModel {
    fn default() -> Self {
        Self {
            engine: EngineSpec::default(),
            transmission: TransmissionSpec::default(),
            aero: AerodynamicSpec::default(),
            current_rpm: 600.0,
            current_gear: 1,
            rolling_coefficient: ROLLING_RESISTANCE_COEFF,
        }
    }
}

impl DrivetrainModel {
    /// Creates a default commercial transit bus drivetrain model.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}
