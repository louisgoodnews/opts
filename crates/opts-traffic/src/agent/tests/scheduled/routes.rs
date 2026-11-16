//! Fixed routes, forks and stops far along a route.

use super::*;

#[test]
fn a_fixed_route_wins_over_the_random_choice_at_a_fork() {
    // Lane 0 ends in a fork: lane 1 straight on, lane 2 a branch to the right.
    let seg = |x: f64, y: f64, h: f64, len: f64| {
        LaneSegment::straight(DVec2::new(x, y), h, len, LaneKind::Road)
    };
    let mut w = world(&[
        seg(0.0, 0.0, 0.0, 100.0),
        seg(0.0, 100.0, 0.0, 100.0),
        seg(0.0, 100.0, 0.12, 100.0),
    ]);
    let succ = w.graph().successors(LaneId(0)).to_vec();
    assert!(succ.len() >= 2, "a fork: {succ:?}");
    for want in &succ {
        let id = w.spawn(LaneId(0), 0.0, 8.0, IdmParams::default()).unwrap();
        w.set_route(id, [*want]);
        let _ = run(&mut w, 15.0);
        let a = w.agent(id).expect("on its way");
        assert_eq!(a.lane, *want, "took the lane of its route");
        w.despawn(id);
    }
}

#[test]
fn the_nearest_place_respects_the_direction_and_the_distance() {
    let w = line();
    let g = w.graph();
    let any = |_: &crate::graph::Lane| true;
    let found = g.nearest_place(DVec2::new(1.5, 250.0), 0.0, 0.5, 5.0, any);
    let (lane, s, d) = found.expect("a lane runs here");
    assert_eq!(lane, LaneId(1));
    assert!((s - 50.0).abs() <= 1.0 && (d - 1.5).abs() < 1e-6, "{s} {d}");
    // Facing the other way, or too far off: nothing.
    assert!(g
        .nearest_place(DVec2::new(1.5, 250.0), std::f64::consts::PI, 0.5, 5.0, any)
        .is_none());
    assert!(g
        .nearest_place(DVec2::new(20.0, 250.0), 0.0, 0.5, 5.0, any)
        .is_none());
}

#[test]
fn a_stop_far_along_the_fixed_route_is_not_missed() {
    // Three lanes of 200 m; the stop is on the third, outside the 150 m plan at first.
    let mut w = world(&[
        straight(0.0, 200.0),
        straight(200.0, 200.0),
        straight(400.0, 200.0),
    ]);
    let id = w.spawn(LaneId(0), 0.0, 8.0, IdmParams::default()).unwrap();
    w.set_route(id, [LaneId(1), LaneId(2)]);
    w.push_stop(id, StopPoint::new(4, LaneId(2), 120.0, 5.0, 0.0));
    let events = run(&mut w, 70.0);
    assert!(!events
        .iter()
        .any(|e| matches!(e, TrafficEvent::StopMissed { .. })));
    assert!(events
        .iter()
        .any(|e| matches!(e, TrafficEvent::ArrivedAtStop { station: 4, .. })));
}
