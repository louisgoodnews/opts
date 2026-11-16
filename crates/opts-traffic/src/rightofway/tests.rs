//! Unit tests for rightofway.

use super::*;
use crate::geometry::PathGeometry;
use crate::graph::{ConnectParams, LaneKind, LaneRules, LaneSegment};
use glam::DVec2;
use std::f64::consts::{FRAC_PI_2, PI};

fn seg(g: PathGeometry) -> LaneSegment {
    LaneSegment {
        geometry: g,
        ..LaneSegment::straight(DVec2::ZERO, 0.0, 1.0, LaneKind::Road)
    }
}

fn straight(x: f64, y: f64, heading: f64) -> LaneSegment {
    seg(PathGeometry::straight(DVec2::new(x, y), heading, 80.0))
}

/// The precedence of lane 0 over lane 1 in lane 0's first conflict zone.
fn p01(segs: &[LaneSegment]) -> Precedence {
    let g = LaneGraph::build(segs, &ConnectParams::default());
    let c = *g
        .conflicts(LaneId(0))
        .iter()
        .find(|c| c.other == LaneId(1))
        .expect("conflict");
    precedence(&g, LaneId(0), &c)
}

#[test]
fn right_before_left() {
    // A north-bound at x = 0; B west-bound along y = 40 comes from A's right (east).
    let a = straight(0.0, 0.0, 0.0);
    let from_right = straight(40.0, 40.0, 3.0 * FRAC_PI_2);
    let from_left = straight(-40.0, 40.0, FRAC_PI_2);
    assert_eq!(p01(&[a.clone(), from_right]), Precedence::Second);
    assert_eq!(p01(&[a, from_left]), Precedence::First);
}

#[test]
fn a_higher_priority_beats_right_before_left_and_unmarked_lanes_rank_in_between() {
    let mut a = straight(0.0, 0.0, 0.0);
    let mut b = straight(40.0, 40.0, 3.0 * FRAC_PI_2); // from A's right
    a.rules = LaneRules {
        priority: Some(192),
        ..LaneRules::default()
    };
    b.rules = LaneRules {
        priority: Some(64),
        ..LaneRules::default()
    };
    assert_eq!(p01(&[a.clone(), b.clone()]), Precedence::First);
    // Only the other lane marked low: the unmarked lane (128) outranks it.
    a.rules = LaneRules::default();
    assert_eq!(p01(&[a.clone(), b.clone()]), Precedence::First);
    // Only the other lane marked high: unmarked A yields although B... is from the right anyway.
    b.rules = LaneRules {
        priority: Some(192),
        ..LaneRules::default()
    };
    assert_eq!(p01(&[a, b]), Precedence::Second);
}

#[test]
fn left_turners_yield_to_oncoming_traffic() {
    // A north-bound turning left (counter-clockwise arc); B oncoming south-bound, straight.
    let left_turn = seg(PathGeometry {
        start: DVec2::new(1.75, -10.0),
        heading: 0.0,
        radius: -9.0,
        length: 9.0 * FRAC_PI_2,
    });
    let oncoming = straight(-1.75, 40.0, PI);
    assert_eq!(
        p01(&[left_turn.clone(), oncoming.clone()]),
        Precedence::Second
    );
    // Seen from the other lane the oncoming vehicle goes first.
    let g = LaneGraph::build(&[oncoming, left_turn], &ConnectParams::default());
    let c = *g
        .conflicts(LaneId(0))
        .iter()
        .find(|c| c.other == LaneId(1))
        .expect("conflict");
    assert_eq!(precedence(&g, LaneId(0), &c), Precedence::First);
}

#[test]
fn merges_and_parallel_flows_are_decided_by_arrival() {
    let g = LaneGraph::build(
        &[
            straight(0.0, 0.0, 0.0),
            straight(0.0, 80.0, 0.0),
            seg(PathGeometry::straight(DVec2::new(-0.2, 30.0), 0.0, 50.0)),
        ],
        &ConnectParams::default(),
    );
    let c = *g
        .conflicts(LaneId(0))
        .iter()
        .find(|c| c.kind == ConflictKind::Merge)
        .expect("merge");
    assert_eq!(precedence(&g, LaneId(0), &c), Precedence::Equal);
}
