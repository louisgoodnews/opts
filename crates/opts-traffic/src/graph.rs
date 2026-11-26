//! The directed lane graph built from OMSI `[path]` records.
//!
//! OMSI never names a path's successor: **connectivity is geometric** — a lane continues into any
//! lane of the same kind whose start coincides with its end in position and heading
//! (`docs/src/formats/text/sco.md`, "Path connectivity"). [`LaneGraph::build`] does exactly that
//! with a spatial hash, after expanding every segment into directed lanes (`direction` `1` is
//! stored reversed, `2` yields two lanes).

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::sync::OnceLock;

use glam::DVec2;

use crate::rng::Rng;

#[cfg(test)]
use build::angle_diff;

mod build;
mod ids;
mod lane;
mod lane_rules;
mod locate;
mod queries;
mod segment;
mod shortest;

pub use ids::{Direction, LaneId, LaneKind};
pub use lane::{ConnectParams, Lane, SpawnError};
pub use lane_rules::{Elevation, LaneRules};
pub use locate::{CourseProgress, CourseState, LaneCandidate, LaneMatch, LaneMatcher, MatchParams};
pub use segment::LaneSegment;

/// Directed lanes with their successors.
#[derive(Debug, Clone, Default)]
pub struct LaneGraph {
    pub(super) lanes: Vec<Lane>,
    pub(super) successors: Vec<Vec<LaneId>>,
    predecessors: Vec<Vec<LaneId>>,
    /// Spatial hash of lane samples for conflict search.
    pub(crate) grid: HashMap<(i64, i64), Vec<u32>>,
    /// Conflict zones per lane, computed on first use.
    pub(crate) conflict_cache: Vec<OnceLock<Vec<crate::conflict::Conflict>>>,
    /// Parallel same-direction lanes per lane, computed on first use.
    pub(crate) neighbour_cache: Vec<OnceLock<Vec<crate::neighbours::Neighbour>>>,
}

impl LaneGraph {
    /// Number of lanes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.lanes.len()
    }

    /// Whether the graph has no lanes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lanes.is_empty()
    }

    /// All lane ids.
    pub fn lane_ids(&self) -> impl Iterator<Item = LaneId> {
        (0..self.lanes.len() as u32).map(LaneId)
    }

    /// The lane `id`.
    #[must_use]
    pub fn lane(&self, id: LaneId) -> Option<&Lane> {
        self.lanes.get(id.0 as usize)
    }

    /// Lanes that continue `id`, sorted by id.
    #[must_use]
    pub fn successors(&self, id: LaneId) -> &[LaneId] {
        self.successors
            .get(id.0 as usize)
            .map_or(&[], Vec::as_slice)
    }

    /// The lane with stable key `key` (see [`Lane::key`]).
    #[must_use]
    pub fn lane_by_key(&self, key: (u64, u32)) -> Option<LaneId> {
        self.lanes
            .iter()
            .position(|l| l.key == key)
            .map(|i| LaneId(i as u32))
    }

    /// Lanes that lead into `id`, sorted by id.
    #[must_use]
    pub fn predecessors(&self, id: LaneId) -> &[LaneId] {
        self.predecessors
            .get(id.0 as usize)
            .map_or(&[], Vec::as_slice)
    }
}

#[cfg(test)]
mod tests;
