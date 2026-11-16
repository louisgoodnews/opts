//! Unit tests of the lane overlay's pure functions.

use super::lanes::*;
use super::*;
use bevy::math::DVec2;
use opts_traffic::LaneKind;
use opts_traffic::{ConnectParams, LaneGraph, LaneId, LaneSegment, PathGeometry};

fn two_lanes() -> LaneGraph {
    let first = PathGeometry::straight(DVec2::ZERO, 0.0, 100.0).end();
    let segments = [
        LaneSegment::straight(DVec2::ZERO, 0.0, 100.0, LaneKind::Road),
        LaneSegment::straight(first, 0.0, 50.0, LaneKind::Road),
    ];
    LaneGraph::build(&segments, &ConnectParams::default())
}

#[test]
fn a_lane_is_sampled_every_step_with_both_ends_on_the_centre_line() {
    let graph = two_lanes();
    let lane = graph.lane(LaneId(0)).expect("lane 0");
    let pts = lane_samples(lane, 10.0);
    assert_eq!(pts.len(), 11, "{pts:?}");
    assert!(pts[0].distance(bevy_point(DVec2::ZERO, 0.0)) < 1e-3);
    let end = bevy_point(lane.geometry.end(), lane.z_at(lane.length()));
    assert!(pts[10].distance(end) < 1e-3, "{pts:?}");
}

#[test]
fn a_successor_arrow_runs_from_the_end_of_a_lane_to_the_start_of_the_next() {
    let graph = two_lanes();
    let succ = graph.successors(LaneId(0));
    assert!(succ.contains(&LaneId(1)), "{succ:?}");
    let (a, b) = successor_arrow(
        graph.lane(LaneId(0)).unwrap(),
        graph.lane(LaneId(1)).unwrap(),
    );
    assert!(
        a.distance(b) < 0.5,
        "a joined pair leaves a short arrow: {a:?} {b:?}"
    );
}

#[test]
fn lanes_beyond_the_radius_are_not_drawn() {
    let graph = two_lanes();
    let lane = graph.lane(LaneId(0)).unwrap();
    assert!(lane_near(lane, DVec2::new(10.0, 10.0)));
    assert!(!lane_near(lane, DVec2::new(5000.0, 5000.0)));
}

#[test]
fn the_status_line_shows_both_layers_and_their_keys() {
    let o = LaneOverlay {
        lanes: true,
        agent_ids: false,
    };
    assert_eq!(layer_status(&o), "[F11 lanes on | Shift+F11 agent ids off]");
}
