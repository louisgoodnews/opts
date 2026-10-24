//! Bends, kinks and speed caps.

use super::*;

/// A straight 100 m, a right-hand quarter circle of radius `r` and another straight.
fn bend(r: f64) -> TrafficWorld {
    use std::f64::consts::FRAC_PI_2;
    let arc = LaneSegment {
        geometry: PathGeometry {
            start: DVec2::new(0.0, 100.0),
            heading: 0.0,
            radius: r,
            length: r * FRAC_PI_2,
        },
        ..straight(100.0, 0.0)
    };
    let after = LaneSegment::straight(DVec2::new(r, 100.0 + r), FRAC_PI_2, 200.0, LaneKind::Road);
    world(&[straight(0.0, 100.0), arc, after])
}

#[test]
fn in_a_tight_bend_the_axles_track_the_lane_and_the_speed_suits_the_radius() {
    let r = 12.0;
    let mut w = bend(r);
    let id = w.spawn(LaneId(0), 0.0, 12.0, IdmParams::default()).unwrap();
    let centre = DVec2::new(r, 100.0);
    let (l, v_c) = (IdmParams::default().wheelbase, (2.5 * r).sqrt());
    let (mut braked_before, mut on_arc) = (false, 0);
    for _ in 0..300 {
        w.step(0.1);
        let Some(a) = w.agent(id) else { break };
        if a.lane == LaneId(0) && a.accel < -0.3 {
            braked_before = true;
        }
        if a.lane == LaneId(1) && a.s > a.params.front_overhang + l {
            on_arc += 1;
            assert!(a.speed <= v_c + 0.3, "{} m/s on the arc", a.speed);
            // Front axle on the lane; the rear axle off-tracks inwards by at most
            // R − √(R² − L²) (≈ 0.29 m) and never outwards.
            let front = a.front_axle.distance(centre);
            let rear = a.rear_axle.distance(centre);
            assert!(
                (front - r).abs() < 0.1,
                "front axle {front} from the centre"
            );
            assert!(
                rear <= r + 0.1 && rear >= (r * r - l * l).sqrt() - 0.1,
                "rear axle {rear} from the centre"
            );
            assert!(a.steer > 0.05, "steers right: {}", a.steer);
        }
    }
    assert!(braked_before, "braking starts before the arc");
    assert!(on_arc > 5);
}

#[test]
fn a_kinked_lane_join_turns_the_body_smoothly() {
    // A 12° kink between two lanes (within the join tolerance): the body's heading must not
    // jump at the join.
    let kink = 12f64.to_radians();
    let next = LaneSegment::straight(DVec2::new(0.0, 100.0), kink, 100.0, LaneKind::Road);
    let mut w = world(&[straight(0.0, 100.0), next]);
    // 25 km/h, a junction speed: the body turns over about a wheelbase of travel.
    let slow = IdmParams {
        desired_speed: 7.0,
        ..IdmParams::default()
    };
    let id = w.spawn(LaneId(0), 70.0, 7.0, slow).unwrap();
    let mut h = w.agent(id).unwrap().heading();
    let mut worst = 0.0f64;
    for _ in 0..80 {
        w.step(0.1);
        let Some(a) = w.agent(id) else { break };
        worst = worst.max((a.heading() - h).abs());
        h = a.heading();
    }
    assert!(
        (h - kink).abs() < 0.01,
        "ends on the new lane's heading: {h}"
    );
    assert!(
        worst < 2f64.to_radians(),
        "heading jumped {}°",
        worst.to_degrees()
    );
}

#[test]
fn a_vehicle_brakes_before_a_lower_limit_and_keeps_it() {
    let mut slow = straight(100.0, 200.0);
    slow.speed_limit = Some(30.0 / 3.6);
    let mut w = world(&[straight(0.0, 100.0), slow]);
    let id = w
        .spawn(LaneId(0), 50.0, 50.0 / 3.6, IdmParams::default())
        .unwrap();
    for _ in 0..200 {
        w.step(0.1);
        let a = w.agent(id).unwrap();
        if a.lane == LaneId(1) {
            assert!(a.speed <= 30.0 / 3.6 + 0.3, "{} km/h", a.speed * 3.6);
        }
    }
    assert_eq!(w.agent(id).unwrap().lane, LaneId(1));
}

#[test]
fn the_speed_cap_is_not_carried_over_from_the_spawn_lane() {
    let mut slow = straight(0.0, 100.0);
    slow.speed_limit = Some(30.0 / 3.6);
    let mut fast = straight(100.0, 600.0);
    fast.speed_limit = Some(70.0 / 3.6);
    let mut w = world(&[slow, fast]);
    let id = w.spawn(LaneId(0), 0.0, 5.0, IdmParams::default()).unwrap();
    for _ in 0..400 {
        w.step(0.1);
    }
    let a = w.agent(id).unwrap();
    assert!(
        a.speed > 60.0 / 3.6,
        "{} km/h on a 70 km/h lane",
        a.speed * 3.6
    );
}

/// A 200 m road that climbs 10 % from a start at 5 m.
fn hill() -> TrafficWorld {
    let climb = LaneSegment {
        elevation: crate::graph::Elevation {
            z0: 5.0,
            grad_start: 0.1,
            grad_end: 0.1,
            rise: None,
        },
        ..straight(0.0, 200.0)
    };
    world(&[climb])
}

#[test]
fn on_a_slope_the_body_lies_along_it_and_the_rear_axle_is_lower_than_the_front() {
    let mut w = hill();
    let p = IdmParams::default();
    let id = w.spawn(LaneId(0), 80.0, 0.0, p.clone()).unwrap();
    let a = w.agent(id).unwrap();
    let (rear_z, pitch) = a.ground_pose(w.graph()).expect("pose");
    assert!((pitch - 0.1_f64.atan()).abs() < 1e-3, "pitch {pitch}");
    // The rear axle is one wheelbase and the front overhang behind the bumper (at s = 80).
    let expected = 5.0 + 0.1 * (80.0 - p.front_overhang - p.wheelbase);
    assert!((rear_z - expected).abs() < 0.02, "{rear_z} vs {expected}");
}

#[test]
fn at_the_start_of_a_lane_the_pitch_is_flat_not_wrong() {
    let mut w = hill();
    let id = w.spawn(LaneId(0), 0.0, 0.0, IdmParams::default()).unwrap();
    let a = w.agent(id).unwrap();
    let (_, pitch) = a.ground_pose(w.graph()).expect("pose");
    assert!(pitch.abs() < 1e-9);
}
