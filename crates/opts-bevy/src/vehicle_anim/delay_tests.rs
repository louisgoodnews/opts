//! `delay` damping of `[newanim]` steps (M-0045).

use super::tests::spin;
use super::*;

fn damped(rate: f32, target: f32, dt: f32, seconds: f32) -> f32 {
    let mut step = spin("needle");
    step.delay = Some(rate);
    let mut anim = MeshAnimation {
        owner: Entity::PLACEHOLDER,
        steps: vec![step],
        visible: Vec::new(),
        shown: Vec::new(),
    };
    anim.follow(&[0.0], dt);
    let frames = (seconds / dt).round() as usize;
    for _ in 0..frames {
        anim.follow(&[target], dt);
    }
    anim.shown[0]
}

#[test]
fn delay_step_response_is_independent_of_the_frame_rate() {
    // A needle with delay 4 /s after 0.5 s: 1 - e^-2 of the step.
    let expect = 10.0 * (1.0 - (-2.0f32).exp());
    for dt in [1.0 / 144.0, 1.0 / 60.0, 1.0 / 30.0, 0.125, 0.5] {
        let v = damped(4.0, 10.0, dt, 0.5);
        assert!((v - expect).abs() < 1e-3, "dt {dt}: {v} vs {expect}");
    }
}

#[test]
fn delay_starts_at_the_first_value_and_settles_without_overshoot() {
    let mut step = spin("needle");
    step.delay = Some(2.0);
    let mut anim = MeshAnimation {
        owner: Entity::PLACEHOLDER,
        steps: vec![step],
        visible: Vec::new(),
        shown: Vec::new(),
    };
    anim.follow(&[7.0], 1.0 / 60.0);
    assert_eq!(anim.shown, [7.0], "no ramp from zero at spawn");
    for _ in 0..2000 {
        anim.follow(&[3.0], 1.0 / 60.0);
        assert!(anim.shown[0] >= 3.0 && anim.shown[0] <= 7.0);
    }
    assert!((anim.shown[0] - 3.0).abs() < 1e-3);
    // A huge frame (a stall) lands on the target, never beyond it.
    anim.follow(&[9.0], 100.0);
    assert!((anim.shown[0] - 9.0).abs() < 1e-4);
}

#[test]
fn no_delay_is_byte_identical_to_the_undamped_follow() {
    let step = spin("w");
    for (shown, target, dt) in [
        (0.0f32, 1.0f32, 0.016f32),
        (3.5, -2.0, 0.5),
        (1.0, 1.0, 0.0),
    ] {
        assert_eq!(step.follow(shown, target, dt).to_bits(), target.to_bits());
    }
    let mut slow = spin("w");
    slow.maxspeed = Some(0.5);
    assert_eq!(slow.follow(0.0, 1.0, 1.0), 0.5);
}

#[test]
fn delay_with_maxspeed_is_limited_by_the_slower_of_the_two() {
    let mut step = spin("w");
    step.delay = Some(50.0);
    step.maxspeed = Some(0.5);
    let v = step.follow(0.0, 1.0, 0.1);
    assert!((v - 0.05).abs() < 1e-6, "{v}");
}
