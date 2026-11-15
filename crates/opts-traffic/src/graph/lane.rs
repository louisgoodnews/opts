//! A directed lane of the graph, the connection tolerances and the spawn errors.

use thiserror::Error;

use crate::geometry::PathGeometry;

use super::*;

/// A directed lane.
#[derive(Debug, Clone, PartialEq)]
pub struct Lane {
    /// Geometry in travel direction.
    pub geometry: PathGeometry,
    /// Which traffic may use it.
    pub kind: LaneKind,
    /// Width in metres.
    pub width: f64,
    /// Speed limit in m/s, if any.
    pub speed_limit: Option<f64>,
    /// The other `[rule]`s of the path.
    pub rules: LaneRules,
    /// Height profile in travel direction.
    pub elevation: Elevation,
    /// The light that governs entering this lane.
    pub signal: Option<crate::signals::SignalKey>,
    /// The path belongs to a scenery object (see [`LaneSegment::junction`]).
    pub junction: bool,
    /// Indicator stated by the path (see [`LaneSegment::indicator`]).
    pub indicator: Option<crate::agent::Blinker>,
    /// The host's tag from the segment.
    pub source: u64,
    /// Stable identity across rebuilds: the segment's `source` and the number of lanes of that
    /// source before this one (reversed twins of `Both` lanes count).
    pub key: (u64, u32),
}

impl Lane {
    /// Length in metres.
    #[must_use]
    pub fn length(&self) -> f64 {
        self.geometry.length
    }

    /// Height after `s` metres.
    #[must_use]
    pub fn z_at(&self, s: f64) -> f64 {
        self.elevation.at(s, self.length())
    }

    /// Heading change from start to end in radians (positive = clockwise = a right turn).
    #[must_use]
    pub fn turn(&self) -> f64 {
        let d = self.geometry.end_heading() - self.geometry.heading;
        (d + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI
    }
}

/// Tolerances for joining lane ends to lane starts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConnectParams {
    /// Largest distance between an end and a start, metres.
    pub position_tolerance: f64,
    /// Largest heading difference, radians.
    pub heading_tolerance: f64,
}

impl Default for ConnectParams {
    /// 0.75 m and 15°: wider than the ≤ 5 cm gaps of well-built splines, tight enough not to
    /// join neighbouring lanes (≥ 2.5 m apart) ❓ not yet calibrated against OMSI.
    fn default() -> Self {
        Self {
            position_tolerance: 0.75,
            heading_tolerance: 15f64.to_radians(),
        }
    }
}

/// Failure to place an agent.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SpawnError {
    /// The lane id does not exist.
    #[error("no such lane")]
    NoSuchLane,
    /// Another agent is too close to the requested place.
    #[error("not enough free room at the requested position")]
    Occupied,
}
