//! Lane changes against the rules: prohibited overtaking sides, truck lanes, fork choice and indicators.

use super::*;

#[test]
fn no_lane_change_is_started_towards_a_prohibited_overtaking_side() {
    // The right lane is the free one; the changer sits on it and the left lane forbids
    // overtaking, so it must not move left past the slow leader.
    let mut left = straight(0.0, 800.0);
    left.rules.no_overtaking = true;
    let right = LaneSegment::straight(DVec2::new(3.5, 0.0), 0.0, 800.0, LaneKind::Road);
    let mut w = world(&[left, right]);
    w.spawn(LaneId(1), 110.0, 3.0, slow()).unwrap();
    w.spawn(LaneId(1), 30.0, 14.0, IdmParams::default())
        .unwrap();
    for _ in 0..300 {
        w.step(0.1);
    }
    assert!(w.drain_events().iter().all(|e| !matches!(
        e,
        TrafficEvent::LaneChanged {
            side: Side::Left,
            ..
        }
    )));
}

#[test]
fn a_car_stays_out_of_a_trucks_lane() {
    // Whitelist reading (2026-10-19): a `trucks` lane is for trucks only.
    let mut right = LaneSegment::straight(DVec2::new(3.5, 0.0), 0.0, 800.0, LaneKind::Road);
    right.rules.trucks = true;
    let mut w = world(&[straight(0.0, 800.0), right]);
    let car = w
        .spawn(LaneId(0), 50.0, 12.0, IdmParams::default())
        .unwrap();
    let mut events = Vec::new();
    for _ in 0..300 {
        w.step(0.1);
        events.extend(w.drain_events());
    }
    assert!(!events.contains(&TrafficEvent::LaneChanged {
        id: car,
        side: Side::Right
    }));
    assert_eq!(w.agent(car).unwrap().lane, LaneId(0));
}

#[test]
fn the_indicator_of_a_crossing_path_wins_over_the_turn_heuristic() {
    // A straight crossing path that says "left" (parameter 12 = 2), and a right-hand bend
    // that says "none" (0): the path's word counts, before and on the path.
    let mut left = straight(100.0, 30.0);
    left.indicator = Some(Blinker::Left);
    let mut w = world(&[straight(0.0, 100.0), left]);
    let id = w.spawn(LaneId(0), 80.0, 5.0, IdmParams::default()).unwrap();
    assert_eq!(w.command(id).unwrap().blinker, Blinker::Left);
    w.agents[0].s = 40.0;
    assert_eq!(
        w.command(id).unwrap().blinker,
        Blinker::Off,
        "too far ahead"
    );
    w.agents[0].lane = LaneId(1);
    w.agents[0].s = 10.0;
    assert_eq!(
        w.command(id).unwrap().blinker,
        Blinker::Left,
        "kept on the path"
    );
    let quiet = LaneSegment {
        geometry: PathGeometry {
            start: DVec2::new(0.0, 100.0),
            heading: 0.0,
            radius: 15.0,
            length: 15.0 * std::f64::consts::FRAC_PI_2,
        },
        indicator: Some(Blinker::Off),
        ..straight(0.0, 0.0)
    };
    let mut w = world(&[straight(0.0, 100.0), quiet]);
    let id = w.spawn(LaneId(0), 80.0, 5.0, IdmParams::default()).unwrap();
    assert_eq!(w.command(id).unwrap().blinker, Blinker::Off);
}

#[test]
fn a_fork_is_taken_on_the_lane_that_is_not_closed_to_the_cars_group() {
    // Lane 0 ends in a fork: lane 1 is closed to density group 0 (a bus lane), lane 2 is open.
    let mut closed = LaneSegment::straight(DVec2::new(0.0, 100.0), 0.1, 200.0, LaneKind::Road);
    closed.rules.density.set(0, 0.0);
    let open = LaneSegment::straight(DVec2::new(0.0, 100.0), -0.1, 200.0, LaneKind::Road);
    let mut w = world(&[straight(0.0, 100.0), closed, open]);
    assert_eq!(
        w.graph().successors(LaneId(0)).len(),
        2,
        "the fork is connected"
    );
    let cars: Vec<_> = (0..8)
        .map(|i| {
            w.spawn(
                LaneId(0),
                5.0 + f64::from(i) * 11.0,
                10.0,
                IdmParams::default(),
            )
            .unwrap()
        })
        .collect();
    for _ in 0..200 {
        w.step(0.1);
    }
    let mut on_open = 0;
    for id in cars {
        if let Some(a) = w.agent(id) {
            assert_ne!(a.lane, LaneId(1), "a car took the lane closed to its group");
            on_open += usize::from(a.lane == LaneId(2));
        }
    }
    assert!(on_open >= 4, "only {on_open} cars reached the open lane");
}
