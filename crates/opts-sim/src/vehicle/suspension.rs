//! Suspension spring-damper compliance and dynamic load transfer across axles.
//!
//! Models static axle load distribution based on center-of-gravity placement,
//! dynamic longitudinal weight transfer during acceleration and braking, and
//! suspension ride-height deflection.

mod body;
mod body_step;
mod static_loads;
mod wheel;

pub use body::BodySuspension;
pub use static_loads::SuspensionModel;
pub use wheel::{WheelSuspension, SPRING_FACTOR_RANGE};

/// Travel of a spring (m) beyond which the bump stop acts (assumed, the files state no travel ❓).
pub const BUMP_STOP_TRAVEL_M: f32 = 0.12;
/// Stiffness of the bump stop (N/m).
const BUMP_STOP_STIFFNESS_N_M: f32 = 2.0e6;
/// Integration substep of the body model (s).
const BODY_SUBSTEP_S: f32 = 0.004;

/// Gravity (m/s²).
const GRAVITY: f32 = 9.81;

#[cfg(test)]
mod tests;
