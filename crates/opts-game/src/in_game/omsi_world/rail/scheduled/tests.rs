use super::*;
use opts_sim::pedestrian::{FootLane, Travel};

fn lane(a: Vec3, b: Vec3) -> FootLane {
    FootLane::new(vec![a, b], Travel::Both, 1.0).unwrap()
}

/// Lane 0: 0..100 m north, lane 1 continues to 200 m, lane 2 branches off at 100 m.
fn net() -> PedestrianNet {
    let mut n = PedestrianNet::default();
    n.insert_tile(
        (0, 0),
        vec![
            lane(Vec3::ZERO, Vec3::new(0.0, 100.0, 0.0)),
            lane(Vec3::new(0.0, 100.0, 0.0), Vec3::new(0.0, 200.0, 0.0)),
            lane(Vec3::new(0.0, 100.0, 0.0), Vec3::new(50.0, 150.0, 0.0)),
        ],
    );
    n
}

fn place(lane: LaneId, s: f32) -> Place {
    Place { lane, s }
}

/// The better of the two directions from the first target, as the spawner chooses.
fn routed(n: &PedestrianNet, targets: &[Place]) -> Option<RailRoute> {
    [1_i8, -1]
        .into_iter()
        .filter_map(|d| build_route(n, (targets[0], d), targets))
        .reduce(|a, b| if better(&b, &a) { b } else { a })
}

#[test]
fn the_route_runs_through_the_stops_the_way_the_head_faces() {
    let n = net();
    let r = routed(&n, &[place(0, 20.0), place(1, 60.0)]).expect("a route");
    assert_eq!(r.states, vec![(0, 1), (1, 1)]);
    assert_eq!(r.stop_index, vec![0, 1]);
    assert!((r.stop_u[1] - 160.0).abs() < 1e-3, "{:?}", r.stop_u);
    // Towards the start of lane 0 there is nothing: the other way round is taken.
    let back = routed(&n, &[place(1, 60.0), place(0, 20.0)]).expect("a route");
    assert_eq!(back.states, vec![(1, -1), (0, -1)]);
}

#[test]
fn a_branch_is_taken_for_the_stop_on_it() {
    let n = net();
    let r = routed(&n, &[place(0, 10.0), place(2, 40.0)]).expect("a route");
    assert_eq!(r.states.last(), Some(&(2, 1)));
}

#[test]
fn a_stop_behind_on_the_same_lane_is_reached_by_facing_the_other_way() {
    let n = net();
    let r = routed(&n, &[place(0, 80.0), place(0, 20.0)]).expect("a route");
    assert_eq!(r.states, vec![(0, -1)]);
    assert_eq!(r.stop_index.len(), 2);
}

#[test]
fn a_route_from_upstream_starts_before_the_stop() {
    let n = net();
    let mut t = Train::new(1, 10.0, -1);
    assert_eq!(t.retreat(30.0, &n), Moved::Full);
    // 30 m back from lane 1 at 10 m heading south: on lane 0, 20 m from its end... walking
    // south means the train comes from the north, so it starts on lane 1 (the lane behind).
    let r = build_route(&n, (place(t.lane, t.s), t.dir), &[place(1, 10.0)]);
    assert!(r.is_some());
}

#[test]
fn the_walking_coordinate_and_its_inverse_agree() {
    let n = net();
    let states = [(0, 1), (1, 1)];
    assert!((walk_u(&n, &states, 1, 30.0) - 130.0).abs() < 1e-3);
    assert_eq!(walk_at(&n, &states, 130.0), Some((1, 30.0)));
    let back = [(1, -1), (0, -1)];
    // Walking south from 160 m: 40 m into the first lane, i.e. arc length 60.
    assert_eq!(walk_at(&n, &back, 40.0), Some((0, 60.0)));
}

#[test]
fn the_distance_to_a_stop_is_found_ahead_on_the_route() {
    let n = net();
    let mut t = Train::new(0, 90.0, 1);
    t.route = [1].into();
    assert_eq!(distance_to(&t, &n, 0, 95.0), Ahead::At(5.0));
    assert_eq!(distance_to(&t, &n, 0, 50.0), Ahead::Gone);
    match distance_to(&t, &n, 1, 40.0) {
        Ahead::At(d) => assert!((d - 50.0).abs() < 3.1, "{d}"),
        other => panic!("{other:?}"),
    }
}
