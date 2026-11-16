//! Unit tests for idm.

use super::*;

#[test]
fn free_road_converges_to_the_desired_speed() {
    let p = IdmParams::default();
    let mut v = 0.0;
    for _ in 0..6000 {
        v += p.acceleration(v, f64::INFINITY, None) * 0.05;
    }
    assert!((v - p.desired_speed).abs() < 0.05, "v = {v}");
}

#[test]
fn speed_limit_caps_the_free_road_speed() {
    let p = IdmParams::default();
    assert!(p.acceleration(5.0, 5.0, None).abs() < 1e-9);
    assert!(p.acceleration(8.0, 5.0, None) < 0.0);
}

#[test]
fn closing_on_a_standing_leader_brakes_and_stops_short() {
    let p = IdmParams::default();
    let (mut v, mut x, leader_rear) = (p.desired_speed, 0.0, 120.0);
    for _ in 0..4000 {
        let a = p.acceleration(v, f64::INFINITY, Some((leader_rear - x, 0.0)));
        let nv = (v + a * 0.05).max(0.0);
        x += (v + nv) / 2.0 * 0.05;
        v = nv;
    }
    let gap = leader_rear - x;
    assert!(v < 0.05, "still moving at {v}");
    assert!(
        gap > 0.5 && gap < 4.0,
        "stopped {gap} m behind the obstacle"
    );
}

#[test]
fn a_leader_at_the_same_speed_is_followed_at_the_desired_headway() {
    let p = IdmParams::default();
    let v = 10.0;
    // Equilibrium gap: s0 + vT, up to the free-road term.
    let s_eq =
        (p.min_gap + v * p.time_headway) / (1.0 - (v / p.desired_speed).powf(p.delta)).sqrt();
    assert!(p.acceleration(v, f64::INFINITY, Some((s_eq, v))).abs() < 1e-9);
}

#[test]
fn lane_speed_is_the_factor_on_the_limit_below_the_top_speed() {
    let p = IdmParams {
        speed_factor: 1.1,
        ..IdmParams::default()
    };
    assert!((p.lane_speed(Some(10.0), OPEN_ROAD_SPEED) - 11.0).abs() < 1e-9);
    assert!((p.lane_speed(None, OPEN_ROAD_SPEED) - 1.1 * OPEN_ROAD_SPEED).abs() < 1e-9);
    let slow = IdmParams {
        desired_speed: 5.0,
        ..p
    };
    assert_eq!(slow.lane_speed(Some(30.0), OPEN_ROAD_SPEED), 5.0);
    // √(2.5 · 12) for a car, √(2.0 · 12) for a lorry; straight is unbounded.
    assert!((p.curve_speed(-12.0) - 30f64.sqrt()).abs() < 1e-9);
    assert!((IdmParams::heavy().curve_speed(12.0) - 24f64.sqrt()).abs() < 1e-9);
    assert!(p.curve_speed(0.0).is_infinite());
}

#[test]
fn braking_is_bounded() {
    let p = IdmParams::default();
    assert_eq!(
        p.acceleration(13.0, f64::INFINITY, Some((0.5, 0.0))),
        -p.max_decel
    );
}
