//! Strategic lane changes: moving over before a lane ends, and changes after a passed crossing.

use super::*;

#[test]
fn a_lane_change_is_possible_after_a_passed_crossing() {
    use std::f64::consts::FRAC_PI_2;
    // Both lanes are crossed at y = 50; the car is 40 m past it behind a slow one.
    let right = LaneSegment::straight(DVec2::new(3.5, 0.0), 0.0, 800.0, LaneKind::Road);
    let cross = LaneSegment::straight(DVec2::new(-50.0, 50.0), FRAC_PI_2, 100.0, LaneKind::Road);
    let mut w = world(&[straight(0.0, 800.0), right, cross]);
    w.spawn(LaneId(1), 170.0, 3.0, slow()).unwrap();
    let fast = w
        .spawn(LaneId(1), 90.0 + 4.4, 12.0, IdmParams::default())
        .unwrap();
    assert!(!w.graph().conflicts(LaneId(1)).is_empty());
    let mut changed = false;
    for _ in 0..150 {
        w.step(0.1);
        changed |= w
            .drain_events()
            .iter()
            .any(|e| matches!(e, TrafficEvent::LaneChanged { id, .. } if *id == fast));
    }
    assert!(changed, "the passed crossing still blocked the change");
}

#[test]
fn a_strategic_change_is_made_before_the_lane_ends() {
    use std::f64::consts::FRAC_PI_2;
    // The left lane ends (no successor); the right one continues into a right turn. The
    // route goes through the neighbour, so the car must move over in time.
    let right = LaneSegment::straight(DVec2::new(3.5, 0.0), 0.0, 200.0, LaneKind::Road);
    let turn = LaneSegment {
        geometry: PathGeometry {
            start: DVec2::new(3.5, 200.0),
            heading: 0.0,
            radius: 15.0,
            length: 15.0 * FRAC_PI_2,
        },
        ..straight(0.0, 0.0)
    };
    let exit = LaneSegment::straight(DVec2::new(18.5, 215.0), FRAC_PI_2, 300.0, LaneKind::Road);
    let mut w = world(&[straight(0.0, 200.0), right, turn, exit]);
    let id = w
        .spawn(LaneId(0), 60.0, 10.0, IdmParams::default())
        .unwrap();
    let s = w
        .agent(id)
        .unwrap()
        .strategic
        .expect("planned through the neighbour");
    assert_eq!((s.from, s.via, s.side), (LaneId(0), LaneId(1), Side::Right));
    let mut events = Vec::new();
    for _ in 0..300 {
        w.step(0.1);
        events.extend(w.drain_events());
    }
    assert!(events.contains(&TrafficEvent::LaneChanged {
        id,
        side: Side::Right
    }));
    assert!(!events
        .iter()
        .any(|e| matches!(e, TrafficEvent::ReachedDeadEnd { .. })));
    assert_eq!(w.agent(id).unwrap().lane, LaneId(3), "took the turn");
}
