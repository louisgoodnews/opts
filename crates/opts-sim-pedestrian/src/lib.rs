//! # `opts-sim-pedestrian`: the pedestrians of the simulation
//!
//! Split out of `opts-sim` (ISS-066, stage 2; epic: `docs/dev/issues/ISS-066-epic-split-opts-sim.md`).
//! The module keeps its path: `opts_sim_pedestrian::pedestrian`, re-exported by `opts-sim` as
//! `opts_sim::pedestrian`.
//!
//! Pure geometry and kinematics: no engine and no file I/O, so it runs headless. Depends only on
//! `opts-core` (the file formats) and `glam`.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod pedestrian;
