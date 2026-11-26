//! Unit tests for neighbours.

use opts_traffic_lanes::graph::{LaneGraph, LaneId};
use opts_traffic_lanes::neighbours::Side;
use opts_traffic_lanes::graph::{ConnectParams, Direction, LaneKind, LaneSegment};
use glam::DVec2;

fn road(x: f64, y: f64, heading: f64, len: f64) -> LaneSegment {
    LaneSegment::straight(DVec2::new(x, y), heading, len, LaneKind::Road)
}

#[test]
fn side_by_side_lanes_are_neighbours_on_the_right_sides() {
    // Two north-bound lanes 3.5 m apart: lane 1 is to the right (east) of lane 0.
    let g = LaneGraph::build(
        &[road(0.0, 0.0, 0.0, 50.0), road(3.5, 0.0, 0.0, 50.0)],
        &ConnectParams::default(),
    );
    let n0 = g.neighbours(LaneId(0));
    assert_eq!(n0.len(), 1);
    assert_eq!((n0[0].lane, n0[0].side), (LaneId(1), Side::Right));
    let n1 = g.neighbours(LaneId(1));
    assert_eq!((n1[0].lane, n1[0].side), (LaneId(0), Side::Left));
    let (s_other, sep) = n0[0].at(20.0).unwrap();
    assert!((s_other - 20.0).abs() < 1e-6 && (sep - 3.5).abs() < 1e-6);
    assert!(n0[0].covers(0.0, 50.0));
}

#[test]
fn opposing_far_or_diverging_lanes_are_not_neighbours() {
    let mut opposing = road(3.5, 50.0, std::f64::consts::PI, 50.0);
    opposing.direction = Direction::Forward;
    let g = LaneGraph::build(
        &[
            road(0.0, 0.0, 0.0, 50.0),
            opposing,
            road(9.0, 0.0, 0.0, 50.0),  // too far
            road(1.0, 0.0, 0.0, 50.0),  // too close (same lane, doubled)
            road(3.5, 0.0, 0.35, 50.0), // 20° off
        ],
        &ConnectParams::default(),
    );
    assert!(
        g.neighbours(LaneId(0)).is_empty(),
        "{:?}",
        g.neighbours(LaneId(0))
    );
}

#[test]
fn curved_lanes_map_positions_by_arc() {
    use opts_traffic_lanes::geometry::PathGeometry;
    // Concentric right-hand arcs, 3.5 m apart: the outer one is longer.
    let inner = PathGeometry {
        start: DVec2::new(0.0, 0.0),
        heading: 0.0,
        radius: 50.0,
        length: 40.0,
    };
    let outer = inner.offset(-3.5); // 3.5 m to the left = larger radius
    let seg = |g| LaneSegment {
        geometry: g,
        ..road(0.0, 0.0, 0.0, 1.0)
    };
    let g = LaneGraph::build(&[seg(inner), seg(outer)], &ConnectParams::default());
    let n = &g.neighbours(LaneId(0))[0];
    assert_eq!(n.side, Side::Left);
    let (s_other, sep) = n.at(20.0).unwrap();
    // Same angle on a bigger radius → farther along.
    assert!(s_other > 20.0 && (sep - 3.5).abs() < 0.2, "{s_other} {sep}");
}
