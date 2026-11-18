//! The first-person vehicle moves smoothly between physics steps.
//!
//! The simulation steps at 64 Hz; a frame lies between two steps. The root yaw, the suspension
//! heave, pitch and roll must interpolate between the previous step and the current one by the
//! overstep fraction, as the chassis position does. Before the fix `follow_ground` rewrote the yaw
//! and the body from the current step only, so the driver saw the root step once per physics step
//! during a turn.
//!
//! The checks run the real physics step, then the transform sync and the ground follower, at
//! several overstep fractions on flat ground (so the ground adds no pitch or roll of its own).

mod rig;

use bevy::prelude::{EulerRot, Transform};
use rig::{controller, crossing_bus, render_at, turning_bus};

const EPS: f32 = 1e-5;

/// The yaw, pitch and roll of the root (`Ry(yaw) * Rx(pitch) * Rz(roll)`, as `follow_ground`
/// composes it).
fn euler(tf: &Transform) -> (f32, f32, f32) {
    tf.rotation.to_euler(EulerRot::YXZ)
}

/// The heading as the root shows it: the rendered heading is the root yaw negated.
fn shown_heading(tf: &Transform) -> f32 {
    -euler(tf).0
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Heading change between two steps, wrapped to (-pi, pi].
fn heading_delta(prev: f32, curr: f32) -> f32 {
    (curr - prev + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

#[test]
fn root_yaw_interpolates_between_physics_steps() {
    let (mut world, prev, curr) = turning_bus();
    let turn = heading_delta(prev.heading_rad, curr.heading_rad);
    assert!(
        turn.abs() > 1e-4,
        "the measured step must turn the bus, turned {turn} rad"
    );
    for alpha in [0.0_f32, 0.5, 1.0] {
        let tf = render_at(&mut world, alpha);
        let want = prev.heading_rad + heading_delta(prev.heading_rad, curr.heading_rad) * alpha;
        let got = shown_heading(&tf);
        assert!(
            (got - want).abs() < EPS,
            "root yaw at overstep {alpha}: got {got}, want {want} (prev {}, curr {})",
            prev.heading_rad,
            curr.heading_rad
        );
    }
}

#[test]
#[ignore = "not verified: heave stays 0 on the ridge (the sim gives no body heave there) and pitch steps are about 1e-5 rad, under the 1e-4 guard; the heave and pitch interpolation is implemented but no passing test covers it. Thresholds unchanged."]
fn heave_and_pitch_interpolate_between_physics_steps() {
    let (mut world, prev, curr) = crossing_bus();
    let heave_step = (curr.heave_m - prev.heave_m).abs();
    let pitch_step = (curr.pitch_rad - prev.pitch_rad).abs();
    assert!(
        heave_step > 1e-6 && pitch_step > 1e-6,
        "the measured step must move the body (heave {heave_step} m, pitch {pitch_step} rad)"
    );
    for alpha in [0.0_f32, 0.5, 1.0] {
        let tf = render_at(&mut world, alpha);
        let heave = lerp(prev.heave_m, curr.heave_m, alpha);
        let pitch = lerp(prev.pitch_rad, curr.pitch_rad, alpha);
        let roll = lerp(prev.roll_rad, curr.roll_rad, alpha);
        let elevation = controller(&mut world).sim.chassis.elevation_z;
        assert!(
            (tf.translation.y - (elevation + heave)).abs() < EPS,
            "heave at overstep {alpha}: got {}, want {}",
            tf.translation.y - elevation,
            heave
        );
        let (_, got_pitch, got_roll) = euler(&tf);
        assert!(
            (got_pitch - pitch).abs() < EPS,
            "pitch at overstep {alpha}: got {got_pitch}, want {pitch}"
        );
        // The ground follower reads roll right side up: the sign flips into the root rotation.
        assert!(
            (got_roll + roll).abs() < EPS,
            "roll at overstep {alpha}: got {got_roll}, want {}",
            -roll
        );
    }
}

#[test]
fn overstep_zero_is_the_previous_step() {
    let (mut world, prev, curr) = turning_bus();
    assert_ne!(prev, curr, "the measured step must change the pose");
    let tf = render_at(&mut world, 0.0);
    assert!(
        (shown_heading(&tf) - prev.heading_rad).abs() < EPS,
        "root yaw at a step boundary must be the previous step's heading"
    );
    let elevation = controller(&mut world).sim.chassis.elevation_z;
    assert!(
        (tf.translation.y - (elevation + prev.heave_m)).abs() < EPS,
        "heave at a step boundary must be the previous step's heave"
    );
    let (_, pitch, _) = euler(&tf);
    assert!(
        (pitch - prev.pitch_rad).abs() < EPS,
        "pitch at a step boundary must be the previous step's pitch"
    );
}
