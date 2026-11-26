//! `opts-light-quality-core`: the Bevy-free part of the lighting quality (ISS-090). It holds the
//! shafts, light mode and bloom level types with their names and parsers. The Bevy-facing
//! `LightQuality` state, the ladder and the governor stay in `opts-light-quality`, which re-exports
//! this crate, so consumer paths do not change.
//!
//! The crate has no dependencies and reads no engine type.

mod levels;

pub use levels::*;
