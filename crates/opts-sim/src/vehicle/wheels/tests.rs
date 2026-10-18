use super::*;

fn wheel() -> WheelState {
    WheelState {
        rolling_radius_m: 0.5,
        ..WheelState::default()
    }
}

/// A free-rolling wheel stays at the road speed and the tyre pushes nothing.
#[test]
fn a_free_wheel_rolls_with_the_road() {
    let mut w = wheel();
    let v = 10.0;
    w.angular_velocity_rad_s = v / 0.5;
    for _ in 0..200 {
        let s = step_wheel(&mut w, 0.0, 0.0, 30_000.0, v, 0.85, 0.01);
        assert!(s.force_n.abs() < 1.0, "{}", s.force_n);
    }
    assert!((w.angular_velocity_rad_s * 0.5 - v).abs() < 0.01);
}

/// Drive torque becomes a force at the tyre of about torque over radius, below the limit.
#[test]
fn drive_torque_pushes_the_vehicle_by_torque_over_radius() {
    let mut w = wheel();
    let mut last = 0.0;
    for _ in 0..300 {
        last = step_wheel(&mut w, 1_000.0, 0.0, 30_000.0, 5.0, 0.85, 0.01).force_n;
    }
    assert!((last - 2_000.0).abs() < 60.0, "{last}");
}

/// More torque than the tyre carries: the wheel spins up and the force stays at the limit.
#[test]
fn a_spinning_wheel_is_held_at_the_friction_limit() {
    let mut w = wheel();
    let mut s = WheelStep::default();
    for _ in 0..300 {
        s = step_wheel(&mut w, 40_000.0, 0.0, 20_000.0, 5.0, 0.85, 0.005);
    }
    assert!(s.saturated);
    assert!((s.force_n - 0.85 * 20_000.0).abs() < 1.0, "{}", s.force_n);
    assert!(
        w.angular_velocity_rad_s * 0.5 > 5.5,
        "it spins faster than the road"
    );
}

/// A wheel braked beyond the limit locks and the force is the limit against the motion.
#[test]
fn a_locked_wheel_slides_at_the_limit_against_the_motion() {
    let mut w = wheel();
    w.angular_velocity_rad_s = 10.0 / 0.5;
    let mut s = WheelStep::default();
    for _ in 0..300 {
        s = step_wheel(&mut w, 0.0, 20_000.0, 20_000.0, 10.0, 0.85, 0.005);
    }
    assert_eq!(w.angular_velocity_rad_s, 0.0, "locked");
    assert!(
        s.saturated && (s.force_n + 0.85 * 20_000.0).abs() < 1.0,
        "{s:?}"
    );
}

/// A brake torque below the limit slows the wheel with the vehicle: the force is the torque over
/// the radius and the wheel does not lock.
#[test]
fn a_moderate_brake_gives_its_torque_over_radius() {
    let mut w = wheel();
    w.angular_velocity_rad_s = 10.0 / 0.5;
    let mut last = 0.0;
    for _ in 0..300 {
        last = step_wheel(&mut w, 0.0, 3_000.0, 30_000.0, 10.0, 0.85, 0.005).force_n;
    }
    assert!((last + 6_000.0).abs() < 200.0, "{last}");
    assert!(w.angular_velocity_rad_s > 5.0);
}

/// The result of a step of 1/30 s and of 1/120 s agree.
#[test]
fn the_force_does_not_depend_on_the_step() {
    let run = |dt: f32| {
        let mut w = wheel();
        w.angular_velocity_rad_s = 8.0 / 0.5;
        let mut f = 0.0;
        for _ in 0..(1.0 / dt) as usize {
            f = step_wheel(&mut w, 500.0, 2_500.0, 25_000.0, 8.0, 0.85, dt).force_n;
        }
        f
    };
    let (a, b) = (run(1.0 / 30.0), run(1.0 / 120.0));
    assert!((a - b).abs() < 0.05 * b.abs().max(100.0), "{a} {b}");
}

/// A held wheel on a slope: the tyre carries the load of the grade with a creep far below a
/// millimetre per second.
#[test]
fn a_held_wheel_carries_the_grade_force_with_negligible_creep() {
    let mut w = wheel();
    // The vehicle slides at 0.5 mm/s uphill-backwards; the brake holds the wheel at rest.
    let v = -0.0005;
    for _ in 0..50 {
        let s = step_wheel(&mut w, 0.0, 30_000.0, 30_000.0, v, 0.85, 0.01);
        assert!(s.force_n > 0.0 && s.force_n <= 0.85 * 30_000.0, "{s:?}");
    }
    assert_eq!(w.angular_velocity_rad_s, 0.0);
}
