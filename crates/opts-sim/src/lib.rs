//! # 🚌 `opts-sim` — Simulation Core for OPTS
//!
//! Pure Rust simulation algorithms and physical models for public transport vehicles,
//! schedules, passenger demand, and route execution.
//!
//! **Architectural Boundary:** `opts-sim` is completely decoupled from the Bevy game engine.
//! It contains deterministic simulation logic suitable for headless execution, dedicated servers,
//! automated testing, and physics evaluation.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod pedestrian;
pub mod rail;
pub mod script;
pub mod vehicle;

// Convenient top-level re-exports
pub use vehicle::articulation::{ArticulationJoint, ArticulationState};
pub use vehicle::axle::{AxleConfig, AxleState, WheelState};
pub use vehicle::chassis::ChassisState;
pub use vehicle::drivetrain::{
    AerodynamicSpec, DrivetrainForces, DrivetrainModel, EngineSpec, TransmissionSpec,
};
pub use vehicle::inputs::{GearPosition, VehicleInputs};
pub use vehicle::suspension::SuspensionModel;
pub use vehicle::trailer::{TrailerKinematics, TrailerState};
pub use vehicle::VehicleSimulation;
