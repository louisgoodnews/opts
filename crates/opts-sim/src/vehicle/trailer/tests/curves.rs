//! Curves: straight driving, turning against the leader, mirrored curves and the joint limit.

use super::*;

#[test]
fn straight_driving_keeps_the_segment_in_line_and_hitched() {
    let mut t = joint(DVec2::ZERO, 0.0);
    for k in 1..=200 {
        let pos = DVec2::new(0.0, f64::from(k) * 0.1);
        let s = t.step(pos, 0.0, 0.01);
        assert!(s.alpha.abs() < 1e-9);
        // The segment's front coupling is on the hitch.
        let hitch = t.hitch(pos, 0.0);
        assert!((s.origin + t.front_offset(s.heading) - hitch).length() < 1e-9);
    }
}

#[test]
fn a_curve_turns_the_segment_against_the_leader_and_it_off_tracks_inwards() {
    let mut t = joint(DVec2::ZERO, 0.0);
    let states = drive_circle(&mut t, 20.0, 5.0, 40.0);
    let last = states[states.len() - 1];
    // Steady state: the angle stops changing …
    let before = states[states.len() - 500].alpha;
    assert!(
        (last.alpha - before).abs() < 0.01,
        "α still moving: {before} → {}",
        last.alpha
    );
    // … the leader turns clockwise, so the segment lags: α < 0 (anticlockwise against it).
    assert!(last.alpha < -0.2 && last.alpha > -1.0, "α = {}", last.alpha);
    // Off-tracking: the segment's rear axle runs on a smaller circle than the hitch.
    let centre = DVec2::new(20.0, 0.0);
    let hitch_r = (t.hitch(
        centre
            + DVec2::new(
                -20.0 * (5.0f64 / 20.0 * 40.0).cos(),
                20.0 * (5.0f64 / 20.0 * 40.0).sin(),
            ),
        5.0 / 20.0 * 40.0,
    ) - centre)
        .length();
    let axle = last.origin + (right(last.heading) * 0.0 + fwd(last.heading) * (1.0 - 8.0));
    assert!(
        (axle - centre).length() < hitch_r,
        "rear axle {} vs hitch {hitch_r}",
        (axle - centre).length()
    );
}

#[test]
fn mirrored_curves_give_mirrored_angles() {
    let mut a = joint(DVec2::ZERO, 0.0);
    let mut b = joint(DVec2::ZERO, 0.0);
    let right_curve = drive_circle(&mut a, 20.0, 5.0, 30.0);
    let left_curve = drive_circle(&mut b, -20.0, 5.0, 30.0);
    let (ra, la) = (
        right_curve.last().unwrap().alpha,
        left_curve.last().unwrap().alpha,
    );
    assert!((ra + la).abs() < 0.02, "{ra} vs {la}");
}

#[test]
fn the_joint_limit_holds() {
    let mut t = joint(DVec2::ZERO, 0.0);
    // Spinning the leader on the spot: the segment cannot follow beyond the limit.
    let mut worst: f64 = 0.0;
    for k in 0..200 {
        let heading = f64::from(k) * 0.03;
        let s = t.step(DVec2::new(0.0, f64::from(k) * 0.02), heading, 0.02);
        worst = worst.max(s.alpha.abs());
    }
    assert!(worst <= 52.5f64.to_radians() + 1e-9, "α reached {worst}");
}
