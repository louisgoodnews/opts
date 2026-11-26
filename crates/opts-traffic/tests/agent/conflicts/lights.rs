//! Signalled conflicts: red and yellow lights, and a waiting vehicle that must not block cross traffic.

use super::*;

/// A lane of 100 m followed by a lane governed by light `(1, 0)`.
fn light_world(program: SignalProgram) -> TrafficWorld {
    let mut gated = straight(100.0, 200.0);
    gated.signal = Some((1, 0));
    let mut w = world(&[straight(0.0, 100.0), gated]);
    w.set_signals(HashMap::from([((1, 0), program)]));
    w.set_clock(0.0);
    w
}

#[test]
fn vehicles_stop_at_a_red_light_and_go_on_green() {
    let mut w = light_world(SignalProgram::new(vec![(0, 20.0), (6, 100.0)], 50.0).unwrap());
    let id = w.spawn(LaneId(0), 0.0, 12.0, IdmParams::default()).unwrap();
    for _ in 0..150 {
        w.step(0.1);
    }
    let a = w.agent(id).unwrap();
    assert_eq!(a.lane, LaneId(0), "ran the red light");
    assert!(
        a.speed < 0.2 && a.s > 90.0 && a.s < 100.0,
        "stopped at s = {}",
        a.s
    );
    for _ in 0..100 {
        w.step(0.1);
    }
    let a = w.agent(id).unwrap();
    assert_eq!(a.lane, LaneId(1));
    assert!(a.speed > 5.0);
}

#[test]
fn a_vehicle_too_close_to_stop_runs_the_yellow() {
    let mut w = light_world(SignalProgram::new(vec![(9, 10.0), (0, 30.0)], 0.0).unwrap());
    let late = w
        .spawn(LaneId(0), 90.0, 14.0, IdmParams::default())
        .unwrap();
    for _ in 0..30 {
        w.step(0.1);
    }
    assert_eq!(
        w.agent(late).unwrap().lane,
        LaneId(1),
        "stopped hard for a yellow"
    );
    assert!(w.agent(late).unwrap().speed > 10.0);
}

#[test]
fn a_vehicle_waiting_at_red_does_not_hold_up_crossing_traffic() {
    use std::f64::consts::FRAC_PI_2;
    // East-bound approach lane, then the signalled lane across the north-bound road at x = 0.
    let approach =
        LaneSegment::straight(DVec2::new(-100.0, 100.0), FRAC_PI_2, 97.0, LaneKind::Road);
    let mut gated =
        LaneSegment::straight(DVec2::new(-3.0, 100.0), FRAC_PI_2, 106.0, LaneKind::Road);
    gated.signal = Some((7, 0));
    let mut w = world(&[straight(0.0, 200.0), approach, gated]);
    w.set_signals(HashMap::from([(
        (7, 0),
        SignalProgram::new(vec![(0, 60.0), (6, 60.0)], 0.0).unwrap(),
    )]));
    w.set_clock(0.0);
    w.spawn(LaneId(1), 95.0, 0.0, IdmParams::default()).unwrap();
    let north = w
        .spawn(LaneId(0), 60.0, 12.0, IdmParams::default())
        .unwrap();
    let mut slowest = f64::INFINITY;
    for _ in 0..80 {
        w.step(0.1);
        if let Some(a) = w.agent(north) {
            if a.s < 110.0 {
                slowest = slowest.min(a.speed);
            }
        }
    }
    assert!(slowest > 9.0, "the crossing car slowed to {slowest} m/s");
}

#[test]
fn the_rowdier_the_driver_the_more_yellow_lights_are_passed() {
    // Yellow for 6 s, then red; the car is 50 m before the line at 14 m/s (3.6 s to the line).
    let passes = |rowdy: f64| {
        let mut w = light_world(SignalProgram::new(vec![(9, 6.0), (0, 60.0)], 0.0).unwrap());
        let params = IdmParams {
            rowdy,
            ..IdmParams::default()
        };
        let id = w.spawn(LaneId(0), 50.0, 14.0, params).unwrap();
        for _ in 0..50 {
            w.step(0.1);
        }
        w.agent(id).is_none_or(|a| a.lane == LaneId(1))
    };
    assert!(
        !passes(0.0),
        "a careful driver stops for a yellow he can stop for"
    );
    assert!(passes(1.0), "the rowdiest driver passes it");
}
