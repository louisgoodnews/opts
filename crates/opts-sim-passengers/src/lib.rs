//! # `opts-sim-passengers`: the passengers of a vehicle
//!
//! Split out of `opts-sim` (ISS-066, stage 1; plan: `docs/dev/crate-split-sim.md`). The module
//! keeps its path: `opts_sim_passengers::passengers`, re-exported by `opts-sim` as
//! `opts_sim::passengers`.
//!
//! Pure simulation logic: no engine and no file I/O, so it runs headless.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod passengers;
