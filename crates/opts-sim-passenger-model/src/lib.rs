//! # `opts-sim-passenger-model`: the passenger model's data and graph
//!
//! Split out of `opts-sim-passengers` (ISS-086, stage 1): the plain data the passenger
//! simulation works on (`spec`), the path graph of a cabin (`graph`) and the random source
//! (`rng`). `opts-sim-passengers` re-exports them, so `opts_sim::passengers::*` paths do not
//! change.
//!
//! Pure data and math: no engine and no file I/O.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod graph;
pub mod rng;
pub mod spec;
