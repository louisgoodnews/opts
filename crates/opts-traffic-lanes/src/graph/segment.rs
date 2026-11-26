//! A path in world space as read from the map, before direction expansion.

use glam::DVec2;

use crate::geometry::PathGeometry;

use super::*;

/// A path in world space as read from the map, before direction expansion.
#[derive(Debug, Clone, PartialEq)]
pub struct LaneSegment {
    /// Geometry in world space.
    pub geometry: PathGeometry,
    /// Which traffic may use it.
    pub kind: LaneKind,
    /// Lane width in metres.
    pub width: f64,
    /// Allowed direction.
    pub direction: Direction,
    /// Speed limit in m/s from a `speedlimit` `[rule]`, if any.
    pub speed_limit: Option<f64>,
    /// The other `[rule]`s of the path.
    pub rules: LaneRules,
    /// Height profile.
    pub elevation: Elevation,
    /// The light that governs entering this lane (`[use_traffic_light]`), if any.
    pub signal: Option<opts_signals::SignalKey>,
    /// The path belongs to a scenery object (junction, crossing): vehicles do not change lanes
    /// on it.
    pub junction: bool,
    /// The indicator a crossing path states (`.sco` `[path]` parameter 12: `0` none, `2` left,
    /// `3` right); `None` when it states none (splines, value `1`) and the turn heuristic applies.
    pub indicator: Option<crate::Blinker>,
    /// Opaque tag for the host (object or spline id) so lanes can be traced back.
    pub source: u64,
}

impl LaneSegment {
    /// A forward-only straight road segment (handy for tests and tools).
    #[must_use]
    pub fn straight(start: DVec2, heading: f64, length: f64, kind: LaneKind) -> Self {
        Self {
            geometry: PathGeometry::straight(start, heading, length),
            kind,
            width: 2.5,
            direction: Direction::Forward,
            speed_limit: None,
            rules: LaneRules::default(),
            elevation: Elevation::default(),
            signal: None,
            junction: false,
            indicator: None,
            source: 0,
        }
    }
}
