//! `opts-spatial-zones-core`: the Bevy-free part of the spatial zones (ISS-092). It holds the
//! hysteresis tracker that turns per-sample coverage into enter and leave transitions, and the
//! 10 Hz sample gate. The Bevy-facing zone model, registry, sampler, overlay and ceiling stay in
//! `opts-spatial-zones`, which re-exports this module, so consumer paths do not change.
//!
//! The crate has no dependencies and reads no engine type.

pub mod tracker;
