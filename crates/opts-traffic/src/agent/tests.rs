//! Unit tests of `agent.rs` (moved out of the file to keep it under the 500-line limit, refactoring cycle 1).

use super::*;
use crate::geometry::PathGeometry;
use crate::graph::LaneRules;
use crate::graph::{ConnectParams, Direction, LaneKind, LaneSegment};
use crate::neighbours::Side;

fn straight(y: f64, len: f64) -> LaneSegment {
    LaneSegment::straight(DVec2::new(0.0, y), 0.0, len, LaneKind::Road)
}

fn world(segments: &[LaneSegment]) -> TrafficWorld {
    let mut w = TrafficWorld::new(LaneGraph::build(segments, &ConnectParams::default()), 3);
    w.set_stuck_after(f64::INFINITY);
    w
}

mod following;

mod conflicts;

mod lane_change;

// --- vehicles outside the traffic (the player's bus) -----------------------------------------

use crate::obstacles::Obstacle;

mod standing;
