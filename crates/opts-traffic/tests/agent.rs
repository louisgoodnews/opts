//! Unit tests of `agent.rs` that use only the public API. Moved out of `src/` by ISS-094.

use std::collections::HashMap;

use glam::DVec2;
use opts_car_following::IdmParams;
use opts_traffic::agent::{StopPoint, TrafficEvent, TrafficWorld};
use opts_traffic::geometry::PathGeometry;
use opts_traffic::graph::{ConnectParams, LaneGraph, LaneId, LaneKind, LaneRules, LaneSegment};
use opts_traffic::neighbours::Side;
use opts_traffic::obstacles::Obstacle;
use opts_traffic::signals::SignalProgram;

fn straight(y: f64, len: f64) -> LaneSegment {
    LaneSegment::straight(DVec2::new(0.0, y), 0.0, len, LaneKind::Road)
}

fn world(segments: &[LaneSegment]) -> TrafficWorld {
    let mut w = TrafficWorld::new(LaneGraph::build(segments, &ConnectParams::default()), 3);
    w.set_stuck_after(f64::INFINITY);
    w
}

fn slow() -> IdmParams {
    IdmParams {
        desired_speed: 3.0,
        ..IdmParams::default()
    }
}

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

mod conflicts;
mod geometry;
mod lane_change;
mod scheduled;
mod standing;
mod weather;
