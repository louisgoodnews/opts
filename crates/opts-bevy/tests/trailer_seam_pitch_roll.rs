//! The trailer's own body and the trailer-front joint agree under yaw, pitch and roll (ISS-075).
//!
//! The body is the trailer's render placement (`segment_placement`, the function the segment
//! system uses). The joint is `joint_model_transform(joint_transform(..))`, the bellow's trailer
//! end. Hand-computed expectation: with the leader level and heading 0, the body places a model
//! point `m` at `H + R·(m − f)` in Bevy space, where `f` is the front coupling (the level
//! placement pins it on the hitch `H` for every tilt) and `R = Ry(−α)·Rx(−β)·Rz(γ)`: the body
//! yaw is `−α` (`α` clockwise), its pitch `−β` and its roll `γ`. The rest point `H + (m − f)` is
//! carried by the joint to `H + Q·(m − f)` with the same `Q`, which
//! `opts_sim::vehicle::trailer_front::tests::pitch_and_roll_match_render` checks against the same
//! formula. So body and joint must agree to float error at every case below, and the front
//! coupling must stay on `H`.

use bevy::math::{DVec3, Quat, Vec3};
use opts_bevy::articulation::{joint_model_transform, segment_placement};
use opts_bevy::{bevy_to_omsi_pos, omsi_to_bevy_pos};
use opts_sim::vehicle::trailer_front::joint_transform;

/// The hitch in the leader's OMSI frame, as in `trailer_front_live.rs`.
const HITCH: DVec3 = DVec3::new(0.0, -4.4, 0.4);

/// The trailer's front coupling in its model frame (OMSI).
const FRONT: Vec3 = Vec3::new(0.0, 1.0, 0.4);

/// Points of the trailer's front face in its model frame (OMSI).
const FACE: [Vec3; 4] = [
    Vec3::new(-1.2, 0.0, 0.0),
    Vec3::new(1.2, 0.2, 1.5),
    Vec3::new(-0.6, 0.0, 1.8),
    Vec3::new(0.6, 0.2, 0.0),
];

/// `(alpha, beta, gamma)` in degrees: yaw clockwise, pitch nose-up, roll right-side-up.
const CASES: [(f32, f32, f32); 5] = [
    (0.0, 0.0, 0.0),
    (12.0, 3.0, -2.0),
    (-25.0, -4.0, 1.5),
    (25.0, 2.0, 2.0),
    (-12.0, -3.0, -1.0),
];

/// Tolerance of the seam, metres (float error of the chain of transforms is about 1e-6).
const TOLERANCE_M: f32 = 1e-4;

/// The OMSI origin whose level placement pins the front coupling on the hitch for a Bevy yaw.
fn origin_for(yaw: f32) -> Vec3 {
    let hitch = omsi_to_bevy_pos(HITCH.as_vec3());
    let level = Quat::from_rotation_y(yaw);
    bevy_to_omsi_pos(hitch - level * omsi_to_bevy_pos(FRONT))
}

/// A model point of the trailer, in Bevy space, under the body placement of
/// `segment_placement` for the clockwise yaw `alpha`, pitch `-beta` and roll `gamma` (radians).
fn body_point(m: Vec3, alpha: f32, beta: f32, gamma: f32) -> Vec3 {
    let (t, r) = segment_placement(origin_for(-alpha), FRONT, -alpha, -beta, gamma);
    t + r * omsi_to_bevy_pos(m)
}

/// The same rest point carried by the trailer-front joint (radians), in Bevy space.
fn joint_point(rest: Vec3, alpha: f32, beta: f32, gamma: f32) -> Vec3 {
    let j = joint_transform(HITCH, f64::from(alpha), f64::from(beta), f64::from(gamma));
    joint_model_transform(j).transform_point(rest)
}

#[test]
fn the_body_and_the_joint_agree_under_yaw_pitch_and_roll() {
    for (a, b, g) in CASES {
        let (a, b, g) = (a.to_radians(), b.to_radians(), g.to_radians());
        for m in FACE {
            let rest = body_point(m, 0.0, 0.0, 0.0);
            let body = body_point(m, a, b, g);
            let joint = joint_point(rest, a, b, g);
            assert!(
                joint.distance(body) < TOLERANCE_M,
                "alpha {a} beta {b} gamma {g} point {m:?}: joint {joint:?} body {body:?}"
            );
        }
    }
}

#[test]
fn the_front_coupling_stays_on_the_hitch_for_every_angle() {
    let hitch = omsi_to_bevy_pos(HITCH.as_vec3());
    for (a, b, g) in CASES {
        let (a, b, g) = (a.to_radians(), b.to_radians(), g.to_radians());
        let front = body_point(FRONT, a, b, g);
        assert!(
            front.distance(hitch) < TOLERANCE_M,
            "alpha {a} beta {b} gamma {g}: {front:?} vs {hitch:?}"
        );
    }
}
