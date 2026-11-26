//! # `opts-sim-rail`: rail-bound vehicles
//!
//! Split out of `opts-sim` (ISS-093, budget split of the facade). The module keeps its path:
//! `opts_sim_rail::{dynamics, train}`, re-exported by `opts-sim` as `opts_sim::rail`.
//!
//! Rail-bound vehicles: trains moving along a track network with bogie-accurate poses, and a
//! longitudinal dynamics model. No road physics: a train has one coordinate along the track.
//! See `docs/src/simulation/rail-vehicles.md`.
//!
//! The track network is the lane network of [`opts_sim_pedestrian`] built from rail (`[path]` type 2)
//! lanes; junctions are lane ends that coincide. Depends only on `opts-sim-pedestrian` and `glam`.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod dynamics;
pub mod train;

pub use dynamics::{ConsistLoad, RailDynamics, RailInput, ScriptForces};
pub use train::{Branch, Moved, SwitchAhead, Train};
