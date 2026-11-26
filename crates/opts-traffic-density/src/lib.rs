//! The lane density model of AI road traffic, split out of `opts-traffic-lanes` (ISS-093).
//!
//! Density curves and per-lane vehicle-group densities, and the deterministic generator
//! ([`rng::Rng`]) that picks among them. Pure Rust and deterministic: no Bevy, no I/O.
//! `opts-traffic-lanes` re-exports both modules, so `opts_traffic_lanes::density` and
//! `opts_traffic_lanes::rng` keep working.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod density;
pub mod rng;
