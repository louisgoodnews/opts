//! `opts-spatial-zones-model`: the Bevy-free zone data of the spatial zones (ISS-092). It holds the
//! zone model as plain data ([`model`]), the covered-section rule ([`ceiling`]), the per-tile
//! bookkeeping ([`registry`]), the debug-overlay geometry ([`overlay`]) and the `zones.query`
//! payload ([`query`]).
//!
//! This crate reads no engine type and depends on no Bevy crate. It does not depend on
//! `opts-spatial-zones-core`: the core re-exports these modules at their old paths, so consumer
//! paths do not change.

pub mod ceiling;
pub mod model;
pub mod overlay;
pub mod query;
pub mod registry;
