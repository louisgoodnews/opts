//! `opts-spatial-zones-core`: the Bevy-free part of the spatial zones (ISS-092). It holds the zone
//! model as plain data ([`model`]), the covered-section rule ([`ceiling`]), the membership step
//! ([`sampler`]), the per-tile bookkeeping ([`registry`]), the debug-overlay geometry ([`overlay`]),
//! the `zones.query` payload ([`query`]), the hysteresis tracker and the 10 Hz sample gate
//! ([`tracker`]).
//!
//! The Bevy-facing parts (the `Component`, `Message` and `Resource` types, the systems, the plugin
//! and the colours) stay in `opts-spatial-zones`, which re-exports this crate's items at the old
//! paths, so consumer paths do not change. This crate reads no engine type and depends on no
//! Bevy crate.

pub mod ceiling;
pub mod model;
pub mod overlay;
pub mod query;
pub mod registry;
pub mod sampler;
pub mod tracker;
