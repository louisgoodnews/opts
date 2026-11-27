//! # `opts-pedestrian-rules`: rules for the walkers of the footpath network
//!
//! Split out as a new crate (ISS-076) so that the walker model in `opts-sim-pedestrian` keeps its
//! budget. The rules read the network and the walkers through their public API and decide per
//! frame: whether a walker holds at a crossing ([`crossing`]), how fast it may walk behind
//! another walker ([`separation`]) and how much detail a walker is drawn with ([`lod`]).
//!
//! Pure Rust: no engine, no file I/O.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod crossing;
pub mod separation;
