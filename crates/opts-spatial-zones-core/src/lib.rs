//! `opts-spatial-zones-core`: the Bevy-free membership logic of the spatial zones (ISS-092). It
//! holds the hysteresis tracker and the 10 Hz sample gate ([`tracker`]) and the membership step
//! ([`sampler`]). The zone data it reads (the model, the covered-section rule, the tile bookkeeping,
//! the overlay geometry and the query payload) lives in `opts-spatial-zones-model` and is
//! re-exported here at the old paths.
//!
//! The Bevy-facing parts (the `Component`, `Message` and `Resource` types, the systems, the plugin
//! and the colours) stay in `opts-spatial-zones`, which re-exports this crate's items at the old
//! paths, so consumer paths do not change. This crate reads no engine type and depends on no
//! Bevy crate.

pub use opts_spatial_zones_model::{ceiling, model, overlay, query, registry};

pub mod sampler;
pub mod tracker;
