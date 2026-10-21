//! Rail-bound vehicles: trains moving along a track network with bogie-accurate poses, and a
//! longitudinal dynamics model. No road physics: a train has one coordinate along the track.
//! See `docs/src/simulation/rail-vehicles.md`.
//!
//! The track network is the lane network of [`crate::pedestrian`] built from rail (`[path]` type 2)
//! lanes; junctions are lane ends that coincide.

pub mod dynamics;
pub mod train;

pub use dynamics::{ConsistLoad, RailDynamics, RailInput, ScriptForces};
pub use train::{Branch, Moved, Train};
