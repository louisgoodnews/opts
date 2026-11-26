//! Unit tests for conflict.

use glam::DVec2;
use opts_traffic_lanes::graph::{LaneGraph, LaneId};
use opts_traffic_lanes::graph::{ConnectParams, Direction, LaneKind, LaneSegment};
use std::f64::consts::FRAC_PI_2;

fn road(x: f64, y: f64, heading: f64, len: f64) -> LaneSegment {
    LaneSegment::straight(DVec2::new(x, y), heading, len, LaneKind::Road)
}

fn graph(segs: &[LaneSegment]) -> LaneGraph {
    LaneGraph::build(segs, &ConnectParams::default())
}

#[test]
fn perpendicular_lanes_conflict_where_they_cross() {
    // North-bound lane at x = 0 and east-bound lane at y = 50 cross at (0, 50).
    let g = graph(&[
        road(0.0, 0.0, 0.0, 100.0),
        road(-50.0, 50.0, FRAC_PI_2, 100.0),
    ]);
    let c = g.conflicts(LaneId(0));
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].other, LaneId(1));
    assert!(
        c[0].enter < 50.0 && c[0].exit > 50.0 && c[0].exit - c[0].enter < 8.0,
        "{c:?}"
    );
    let d = g.conflicts(LaneId(1));
    assert!(d[0].enter < 50.0 && d[0].exit > 50.0);
    assert_eq!(d[0].other, LaneId(0));
}

#[test]
fn parallel_opposing_and_diverging_lanes_do_not_conflict() {
    let mut both = road(0.0, 0.0, 0.0, 100.0);
    both.direction = Direction::Both;
    // A neighbour 3 m to the side and a lane that leaves at 10°.
    let g = graph(&[
        both,
        road(3.0, 0.0, 0.0, 100.0),
        road(0.0, 0.0, 0.17, 100.0),
    ]);
    for id in g.lane_ids() {
        assert!(
            g.conflicts(id).iter().all(|c| c.other == id),
            "lane {id:?}: {:?}",
            g.conflicts(id)
        );
    }
}

#[test]
fn lanes_into_the_same_successor_merge() {
    let g = graph(&[
        road(0.0, 0.0, 0.0, 50.0),
        road(0.0, 50.0, 0.0, 50.0),
        // A second feeder from the west, ending at (0, 50) heading north.
        road(-0.3, 20.0, 0.0, 30.3),
    ]);
    let c = g.conflicts(LaneId(0));
    assert!(
        c.iter().any(|c| c.other == LaneId(2) && c.exit > 49.9),
        "{c:?}"
    );
}

#[test]
fn unknown_lanes_have_no_conflicts() {
    assert!(graph(&[]).conflicts(LaneId(3)).is_empty());
}
