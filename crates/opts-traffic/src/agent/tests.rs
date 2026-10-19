//! Unit tests of `agent.rs` (moved out of the file to keep it under the 500-line limit, refactoring cycle 1).

use super::*;
use crate::geometry::PathGeometry;
use crate::graph::LaneRules;
use crate::graph::{ConnectParams, Direction, LaneKind, LaneSegment};
use crate::neighbours::Side;
use crate::signals::SignalProgram;

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

mod geometry;

#[test]
fn same_seed_same_run() {
    let run = || {
        let mut w = world(&[
            straight(0.0, 50.0),
            straight(50.0, 50.0),
            straight(50.0, 80.0),
        ]);
        let id = w.spawn(LaneId(0), 0.0, 5.0, IdmParams::default()).unwrap();
        for _ in 0..100 {
            w.step(0.1);
        }
        w.agent(id).map(|a| (a.lane, a.s.to_bits()))
    };
    assert_eq!(run(), run());
}

// --- vehicles outside the traffic (the player's bus) -----------------------------------------

use crate::obstacles::Obstacle;

mod standing;
