//! The read-only road supplier of the mini map (moved from `opts-game`'s `hud::minimap::source`).
//!
//! The map never reads the tile code. A [`MapSource`] hands over the road paths near a point. The
//! lane graph of the traffic state implements it here; `opts-game` re-exports these items as
//! `hud::minimap::{MapSource, MapRoad, may_touch}`.

use bevy::math::DVec2;
use opts_traffic::PathGeometry;

/// One road path in the world frame (x east, y north, metres; compass headings).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapRoad {
    /// Straight line or circular arc.
    pub geometry: PathGeometry,
    /// Width in metres.
    pub width: f64,
}

/// A read-only supplier of road paths.
pub trait MapSource {
    /// Pushes the paths that may touch the circle (`centre`, `radius_m`). Paths further away may
    /// be left out; a few extra ones are fine (the drawing clips).
    fn roads_near(&self, centre: DVec2, radius_m: f64, out: &mut Vec<MapRoad>);
}

/// Whether a path can touch the circle: its start is within `radius + length` of the centre.
#[must_use]
pub fn may_touch(road: &PathGeometry, centre: DVec2, radius_m: f64) -> bool {
    road.start.distance(centre) <= radius_m + road.length.max(0.0)
}
