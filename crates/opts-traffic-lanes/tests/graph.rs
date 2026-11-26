//! Unit tests of `graph.rs`: segment joins, rules, elevation and keys.

use opts_traffic_lanes::graph::{ConnectParams, Direction, Elevation, LaneGraph, LaneId, LaneKind, LaneRules, LaneSegment};
use glam::DVec2;

fn road(x: f64, y: f64, heading: f64, len: f64) -> LaneSegment {
    LaneSegment::straight(DVec2::new(x, y), heading, len, LaneKind::Road)
}

#[test]
fn end_to_start_joins_and_gaps_do_not() {
    let g = LaneGraph::build(
        &[
            road(0.0, 0.0, 0.0, 50.0),
            road(0.0, 50.2, 0.0, 50.0),
            road(0.0, 120.0, 0.0, 10.0),
        ],
        &ConnectParams::default(),
    );
    assert_eq!(g.successors(LaneId(0)), &[LaneId(1)]);
    assert!(g.successors(LaneId(1)).is_empty());
}

#[test]
fn heading_must_match() {
    // Second lane starts at the right place but points east.
    let g = LaneGraph::build(
        &[
            road(0.0, 0.0, 0.0, 50.0),
            road(0.0, 50.0, std::f64::consts::FRAC_PI_2, 20.0),
        ],
        &ConnectParams::default(),
    );
    assert!(g.successors(LaneId(0)).is_empty());
}

#[test]
fn kinds_do_not_mix() {
    let mut ped = road(0.0, 50.0, 0.0, 10.0);
    ped.kind = LaneKind::Pedestrian;
    let g = LaneGraph::build(&[road(0.0, 0.0, 0.0, 50.0), ped], &ConnectParams::default());
    assert!(g.successors(LaneId(0)).is_empty());
}

#[test]
fn vehicle_type_rules_are_a_whitelist() {
    // Project lead 2026-10-19: a type rule means only those types are allowed.
    use opts_car_following::VehicleClass::{Bus, Car, Truck};
    let none = LaneRules::default();
    assert!([Car, Truck, Bus].iter().all(|c| none.allows(*c)));
    let bus_lane = LaneRules { bus: true, ..none };
    assert_eq!(
        [
            bus_lane.allows(Car),
            bus_lane.allows(Truck),
            bus_lane.allows(Bus)
        ],
        [false, false, true]
    );
    let truck_lane = LaneRules {
        trucks: true,
        ..none
    };
    assert_eq!(
        [
            truck_lane.allows(Car),
            truck_lane.allows(Truck),
            truck_lane.allows(Bus)
        ],
        [false, true, false]
    );
    let both = LaneRules {
        bus: true,
        trucks: true,
        ..none
    };
    assert_eq!(
        [both.allows(Car), both.allows(Truck), both.allows(Bus)],
        [false, true, true]
    );
    let no_cars = LaneRules {
        no_cars: true,
        ..none
    };
    assert_eq!(
        [
            no_cars.allows(Car),
            no_cars.allows(Truck),
            no_cars.allows(Bus)
        ],
        [false, true, true]
    );
}

#[test]
fn elevation_follows_the_gradients_and_reverses() {
    let e = Elevation {
        z0: 10.0,
        grad_start: 0.02,
        grad_end: 0.06,
        rise: None,
    };
    // rise = length · (g0 + g1) / 2 = 100 · 0.04 = 4 m.
    assert!((e.at(100.0, 100.0) - 14.0).abs() < 1e-9);
    assert!((e.at(50.0, 100.0) - (10.0 + 50.0 * 0.03)).abs() < 1e-9);
    let back = e.reversed(100.0);
    assert!((back.z0 - 14.0).abs() < 1e-9);
    assert!((back.at(100.0, 100.0) - 10.0).abs() < 1e-9);
    assert!((back.at(50.0, 100.0) - e.at(50.0, 100.0)).abs() < 1e-9);
}

#[test]
fn spline_h_elevation_is_the_mesh_ramp_and_reverses() {
    // Flat ends, 2 m up over 40 m: the S-shaped ramp of the mesh, not a linear gradient.
    let e = Elevation {
        z0: 5.0,
        grad_start: 0.0,
        grad_end: 0.0,
        rise: Some(2.0),
    };
    assert!((e.at(40.0, 40.0) - 7.0).abs() < 1e-9);
    assert!((e.at(20.0, 40.0) - 6.0).abs() < 1e-9);
    assert!(e.at(4.0, 40.0) - 5.0 < 0.06, "starts flat");
    let back = e.reversed(40.0);
    for s in [0.0, 7.0, 20.0, 33.0, 40.0] {
        assert!((back.at(40.0 - s, 40.0) - e.at(s, 40.0)).abs() < 1e-9);
    }
}

#[test]
fn keys_survive_a_rebuild_with_other_segments() {
    let mut a = road(0.0, 0.0, 0.0, 30.0);
    a.source = 5;
    a.direction = Direction::Both;
    let mut b = road(0.0, 100.0, 0.0, 30.0);
    b.source = 6;
    let g1 = LaneGraph::build(&[a.clone(), b.clone()], &ConnectParams::default());
    let g2 = LaneGraph::build(&[b, a], &ConnectParams::default());
    assert_eq!(g1.lane_by_key((5, 1)), Some(LaneId(1)));
    assert_eq!(g2.lane_by_key((5, 1)), Some(LaneId(2)));
    assert_eq!(g2.lane_by_key((9, 0)), None);
}

mod spots;
