//! M-0073 validation: braking, riding and holding of vehicles that state different masses, axle
//! layouts and ratings, against plausible orders of magnitude (documented in
//! `docs/src/simulation/vehicle-dynamics.md`).

#[path = "vehicle_dynamics_validation/body.rs"]
mod body;
#[path = "vehicle_dynamics_validation/braking.rs"]
mod braking;
#[path = "vehicle_dynamics_validation/support.rs"]
mod support;
