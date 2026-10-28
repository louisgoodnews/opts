//! Speed-dependent zoom filter tests for the mini-map.

#[cfg(test)]
use super::super::zoom::*;

/// Runs the filter for `seconds` at a constant speed in ticks of `dt`.
fn run(f: &mut ZoomFilter, seconds: f32, dt: f32, speed: f32, far: f32) -> f32 {
    let mut r = 0.0;
    for _ in 0..(seconds / dt).round() as usize {
        r = f.step(dt, speed, far);
    }
    r
}

#[test]
fn the_target_is_near_when_slow_far_when_fast_and_monotone_between() {
    for far in [100.0, 200.0, 500.0, 1000.0] {
        let near = near_radius(far);
        assert!((target_radius(0.0, far) - near).abs() < 1e-3);
        assert!((target_radius(V_SLOW_KMH, far) - near).abs() < 1e-3);
        assert!((target_radius(V_FAST_KMH, far) - far).abs() < 1e-3);
        assert!(
            (target_radius(130.0, far) - far).abs() < 1e-3,
            "no growth beyond"
        );
        let mut last = 0.0;
        for kmh in 0..=70 {
            let r = target_radius(kmh as f32, far);
            assert!(r >= last - 1e-4, "monotone at {kmh} km/h");
            last = r;
        }
    }
}

#[test]
fn the_near_radius_has_a_floor_and_never_exceeds_the_far_one() {
    assert_eq!(near_radius(100.0), NEAR_MIN_M);
    assert!((near_radius(200.0) - 200.0 / 3.0).abs() < 1e-3);
    assert!((near_radius(1000.0) - 1000.0 / 3.0).abs() < 1e-3);
    assert_eq!(near_radius(30.0), 30.0, "a tiny far radius is not enlarged");
}

#[test]
fn the_first_step_starts_at_the_target_without_animation() {
    let mut f = ZoomFilter::default();
    assert_eq!(f.radius_m(), None);
    let r = f.step(0.1, 0.0, 500.0);
    assert!((r - near_radius(500.0)).abs() < 1e-3);
    let mut g = ZoomFilter::default();
    assert!((g.step(0.1, 80.0, 500.0) - 500.0).abs() < 1e-3);
}

#[test]
fn constant_speed_converges_and_then_stays_put() {
    let mut f = ZoomFilter::default();
    f.step(0.1, 0.0, 500.0);
    let reached = run(&mut f, 15.0, 0.1, 60.0, 500.0);
    assert!((reached / 500.0 - 1.0).abs() < 0.03, "{reached}");
    let (mut lo, mut hi) = (f32::MAX, 0.0f32);
    for _ in 0..100 {
        let r = f.step(0.1, 60.0, 500.0);
        lo = lo.min(r);
        hi = hi.max(r);
    }
    assert!(
        (hi - lo) / hi < 0.01,
        "steady radius moves by {}",
        (hi - lo) / hi
    );
}

#[test]
fn zooming_out_is_faster_than_zooming_in() {
    let mut out = ZoomFilter::default();
    out.step(0.1, 0.0, 500.0);
    let r_out = run(&mut out, 1.5, 0.1, 80.0, 500.0);
    let out_progress =
        (r_out.ln() - near_radius(500.0).ln()) / (500.0f32.ln() - near_radius(500.0).ln());
    let mut inn = ZoomFilter::default();
    inn.step(0.1, 80.0, 500.0);
    let r_in = run(&mut inn, 1.5, 0.1, 0.0, 500.0);
    let in_progress = (500.0f32.ln() - r_in.ln()) / (500.0f32.ln() - near_radius(500.0).ln());
    assert!(
        out_progress > in_progress + 0.1,
        "out {out_progress:.2} in {in_progress:.2}"
    );
}

#[test]
fn a_stop_from_cruise_reaches_the_near_radius_within_eight_seconds() {
    let mut f = ZoomFilter::default();
    f.step(0.1, 60.0, 500.0);
    let r = run(&mut f, 8.0, 0.1, 0.0, 500.0);
    assert!(r / near_radius(500.0) < 1.05, "{r}");
}

#[test]
fn the_result_does_not_depend_on_the_tick_length() {
    let trajectory = |dt: f32| {
        let mut f = ZoomFilter::default();
        f.step(dt, 0.0, 500.0);
        run(&mut f, 3.0, dt, 60.0, 500.0)
    };
    let reference = trajectory(0.05);
    for dt in [0.1, 0.25] {
        let r = trajectory(dt);
        assert!(
            (r / reference - 1.0).abs() < 0.03,
            "dt {dt}: {r} vs {reference}"
        );
    }
}

#[test]
fn jitter_of_a_few_km_h_at_low_speed_does_not_move_the_radius() {
    let mut f = ZoomFilter::default();
    f.step(0.1, 0.5, 500.0);
    run(&mut f, 10.0, 0.1, 0.5, 500.0);
    let (mut lo, mut hi) = (f32::MAX, 0.0f32);
    for i in 0..200 {
        let kmh = if i % 2 == 0 { 0.0 } else { 3.0 };
        let r = f.step(0.1, kmh, 500.0);
        lo = lo.min(r);
        hi = hi.max(r);
    }
    assert!((hi - lo) / hi < 0.01, "radius moved by {}", (hi - lo) / hi);
}

#[test]
fn reset_forgets_and_a_teleport_is_not_a_speed() {
    let mut f = ZoomFilter::default();
    f.step(0.1, 60.0, 500.0);
    f.reset();
    assert_eq!(f.radius_m(), None);
    assert_eq!(
        camera_speed_kmh(5_000.0, 0.1),
        None,
        "a jump is not a speed"
    );
    assert_eq!(camera_speed_kmh(1.0, 0.0), None);
    let v = camera_speed_kmh(1.0, 0.1).expect("a normal move");
    assert!((v - 36.0).abs() < 1e-3, "10 m/s is 36 km/h, got {v}");
}

#[test]
fn the_mode_setting_parses_and_toggles() {
    use super::super::ZoomMode;
    assert_eq!(ZoomMode::parse("speed"), Some(ZoomMode::Speed));
    assert_eq!(ZoomMode::parse("fixed"), Some(ZoomMode::Fixed));
    assert_eq!(ZoomMode::parse("x"), None);
    assert_eq!(ZoomMode::Speed.toggled(), ZoomMode::Fixed);
    assert_eq!(ZoomMode::default(), ZoomMode::Speed);
}
