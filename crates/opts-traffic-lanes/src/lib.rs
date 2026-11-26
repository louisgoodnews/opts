//! The lane graph of AI road traffic, split out of `opts-traffic` (ISS-085).
//!
//! Pure Rust and deterministic: no Bevy, no I/O. `opts-traffic` re-exports every module of this
//! crate, so `opts_traffic::graph` and the other paths keep working.

#![deny(unsafe_code)]
#![warn(missing_docs)]

mod blinker;

pub use blinker::Blinker;
