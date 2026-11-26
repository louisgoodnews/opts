//! Unit tests of `graph.rs`: segment joins, rules, elevation and keys.

use super::*;

fn road(x: f64, y: f64, heading: f64, len: f64) -> LaneSegment {
    LaneSegment::straight(DVec2::new(x, y), heading, len, LaneKind::Road)
}

#[test]
fn directions_expand_to_directed_lanes() {
    let mut both = road(0.0, 0.0, 0.0, 30.0);
    both.direction = Direction::Both;
    let mut rev = road(0.0, 100.0, 0.0, 30.0);
    rev.direction = Direction::Reverse;
    let g = LaneGraph::build(&[both, rev], &ConnectParams::default());
    assert_eq!(g.len(), 3);
    // The reversed lane starts at the original end and heads south.
    let l = g.lane(LaneId(2)).unwrap();
    assert!(l.geometry.start.abs_diff_eq(DVec2::new(0.0, 130.0), 1e-9));
    assert!(angle_diff(l.geometry.heading, std::f64::consts::PI) < 1e-9);
}
