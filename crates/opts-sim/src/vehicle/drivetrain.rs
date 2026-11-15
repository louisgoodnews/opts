//! Powertrain, transmission gearing, braking dynamics, and aerodynamic resistances.
//!
//! Provides a standalone, deterministic vehicle powertrain simulation supporting
//! turbodiesel engine torque curves, automatic gear ratio selection, retarder braking,
//! pneumatic service friction brakes, and aerodynamic drag.

mod model;
mod specs;
mod step;

#[cfg(test)]
mod tests;

pub use model::{DrivetrainForces, DrivetrainModel, ROLLING_RESISTANCE_COEFF};
pub use specs::{AerodynamicSpec, EngineSpec, TransmissionSpec};
