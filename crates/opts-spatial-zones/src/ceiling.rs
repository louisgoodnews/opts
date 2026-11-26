//! The covered-section rule, the facade over `opts-spatial-zones-core::ceiling` (ISS-092). The
//! rule and its constants are pure and live in the core; this module returns the Bevy zone
//! component so the tile loader can spawn it.

use super::model::{SpatialZone, ZoneId};

pub use opts_spatial_zones_core::ceiling::{
    ceiling_half_width, is_overhead_wire, SplineCandidate, MIN_CEILING_HEIGHT_M,
    MIN_CEILING_WIDTH_M, MIN_COVERED_LENGTH_M,
};

/// The zone of one covered spline, or `None` (see the core's `covered_zone`).
#[must_use]
pub fn covered_zone(id: ZoneId, candidate: &SplineCandidate) -> Option<SpatialZone> {
    opts_spatial_zones_core::ceiling::covered_zone(id, candidate).map(SpatialZone::from)
}

/// The zones of a tile's splines: one per covered spline, numbered in order.
#[must_use]
pub fn covered_zones(splines: &[SplineCandidate]) -> Vec<SpatialZone> {
    opts_spatial_zones_core::ceiling::covered_zones(splines)
        .into_iter()
        .map(SpatialZone::from)
        .collect()
}

#[cfg(test)]
use super::model::ZoneKind;

#[cfg(test)]
#[path = "ceiling_tests.rs"]
mod tests;
