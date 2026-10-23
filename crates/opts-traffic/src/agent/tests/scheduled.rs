//! Scheduled vehicles: fixed routes and stops with a dwell and a scheduled departure.

use super::*;

/// Lanes 0 (north, 0-200 m) and 1 (north, 200-400 m), end to end.
fn line() -> TrafficWorld {
    world(&[straight(0.0, 200.0), straight(200.0, 200.0)])
}

fn run(w: &mut TrafficWorld, seconds: f64) -> Vec<TrafficEvent> {
    let mut events = Vec::new();
    for _ in 0..(seconds * 10.0) as usize {
        w.step(0.1);
        events.extend(w.drain_events());
    }
    events
}

#[test]
fn a_vehicle_stops_at_its_stop_point_and_leaves_at_the_scheduled_time() {
    let mut w = line();
    w.set_clock(1000.0);
    let id = w.spawn(LaneId(0), 0.0, 8.0, IdmParams::default()).unwrap();
    // Stop at 150 m of lane 0: at least 10 s, not before clock 1050.
    w.push_stop(id, StopPoint::new(7, LaneId(0), 150.0, 10.0, 1050.0));
    let events = run(&mut w, 30.0);
    let arrived = events
        .iter()
        .find_map(|e| match e {
            TrafficEvent::ArrivedAtStop {
                station: 7, clock, ..
            } => Some(*clock),
            _ => None,
        })
        .expect("arrived");
    assert!(arrived > 1000.0 && arrived < 1040.0, "arrival {arrived}");
    // Still standing at the stop while the scheduled departure has not come.
    let a = w.agent(id).expect("still there");
    assert!(
        a.lane == LaneId(0) && a.speed < 0.5 && (a.s - 150.0).abs() < 1.5,
        "{a:?}"
    );
    assert!(!events
        .iter()
        .any(|e| matches!(e, TrafficEvent::DepartedStop { .. })));
    // Past the departure time it leaves, not earlier.
    let later = run(&mut w, 40.0);
    let left = later
        .iter()
        .find_map(|e| match e {
            TrafficEvent::DepartedStop {
                station: 7, clock, ..
            } => Some(*clock),
            _ => None,
        })
        .expect("departed");
    assert!((1050.0..1051.0).contains(&left), "departure {left}");
    assert!(w
        .agent(id)
        .is_none_or(|a| a.lane != LaneId(0) || a.s > 160.0));
}

#[test]
fn the_dwell_holds_a_vehicle_that_arrives_late() {
    let mut w = line();
    w.set_clock(2000.0);
    let id = w.spawn(LaneId(0), 0.0, 8.0, IdmParams::default()).unwrap();
    // The scheduled departure has long passed: only the 20 s dwell holds it.
    w.push_stop(id, StopPoint::new(3, LaneId(0), 100.0, 20.0, 0.0));
    let events = run(&mut w, 60.0);
    let (mut arrived, mut left) = (None, None);
    for e in &events {
        match e {
            TrafficEvent::ArrivedAtStop { clock, .. } => arrived = Some(*clock),
            TrafficEvent::DepartedStop { clock, .. } => left = Some(*clock),
            _ => {}
        }
    }
    let (arrived, left) = (arrived.unwrap(), left.unwrap());
    assert!(
        left - arrived >= 20.0 && left - arrived < 21.0,
        "{arrived} {left}"
    );
}

#[test]
fn stops_are_served_in_order() {
    let mut w = line();
    w.set_clock(0.0);
    let id = w.spawn(LaneId(0), 0.0, 8.0, IdmParams::default()).unwrap();
    w.push_stop(id, StopPoint::new(1, LaneId(0), 100.0, 5.0, 0.0));
    w.push_stop(id, StopPoint::new(2, LaneId(1), 80.0, 5.0, 0.0));
    let events = run(&mut w, 80.0);
    let order: Vec<u32> = events
        .iter()
        .filter_map(|e| match e {
            TrafficEvent::DepartedStop { station, .. } => Some(*station),
            _ => None,
        })
        .collect();
    assert_eq!(order, vec![1, 2]);
}

#[test]
fn a_stop_on_a_lane_the_vehicle_is_past_is_dropped() {
    let mut w = line();
    let id = w.spawn(LaneId(1), 50.0, 8.0, IdmParams::default()).unwrap();
    w.push_stop(id, StopPoint::new(9, LaneId(0), 100.0, 5.0, 0.0));
    let events = run(&mut w, 2.0);
    assert!(events
        .iter()
        .any(|e| matches!(e, TrafficEvent::StopMissed { station: 9, .. })));
    assert!(w.agent(id).is_some_and(|a| a.stops.is_empty()));
}

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
