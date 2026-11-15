//! Joint limits and the bellows: beta and gamma follow the coupling character.

use super::*;

#[test]
pub(super) fn beta_is_limited_to_the_coupling_character() {
    let mut seg = ArticulatedSegment::new(Vec2::ZERO, Vec2::ZERO, -5.0, 52.5);
    assert_eq!(seg.clamp_beta_deg(35.0), 35.0, "unlimited without limits");
    seg.beta_limits_deg = Some((-20.0, 20.0));
    assert_eq!(seg.clamp_beta_deg(35.0), 20.0);
    assert_eq!(seg.clamp_beta_deg(-35.0), -20.0);
    assert_eq!(seg.clamp_beta_deg(5.0), 5.0);
}

/// The bellows' follower end (behind the joint) must land where the follower's body puts it: a
/// follower pitched nose up by `p` lowers its front-rear points behind the joint, and the bellows
/// dummy (`anim_rot articulation_0_beta`, no origin rotation, factor 1) must do the same.
#[test]
pub(super) fn bellows_beta_moves_the_follower_end_like_the_follower_body() {
    use opts_bevy::{animation_matrix, joint_beta_deg, AnimStep};
    let p = 8.0_f32.to_radians();
    let step = AnimStep {
        frame: Mat4::IDENTITY,
        variable: "articulation_0_beta".into(),
        factor: 1.0,
        translate: false,
        maxspeed: None,
        delay: None,
    };
    let beta = joint_beta_deg(0.0, p);
    let end_by_bellows =
        animation_matrix(&[step], |_| beta).transform_point3(Vec3::new(0.0, 0.0, 2.0));
    // Bevy axes: -z forward, nose-up pitch is a positive rotation about x.
    let end_by_body = Quat::from_rotation_x(p) * Vec3::new(0.0, 0.0, 2.0);
    assert!(
        end_by_bellows.distance(end_by_body) < 1e-4,
        "bellows {end_by_bellows:?}, body {end_by_body:?}"
    );
}

/// Gamma (roll, right side up positive) uses the frame `origin_rot (0,90,90)` of the census: the
/// bellows end must roll like the body (no sign change needed).
#[test]
pub(super) fn bellows_gamma_rolls_the_follower_end_like_the_follower_body() {
    use opts_bevy::{animation_matrix, AnimStep};
    let r = 6.0_f32.to_radians();
    let frame = Mat4::from_rotation_x(0.0)
        * Mat4::from_rotation_y(90.0_f32.to_radians())
        * Mat4::from_rotation_z(90.0_f32.to_radians());
    let step = AnimStep {
        frame,
        variable: "articulation_0_gamma".into(),
        factor: 1.0,
        translate: false,
        maxspeed: None,
        delay: None,
    };
    let gamma = (r - 0.0).to_degrees();
    let by_bellows = animation_matrix(&[step], |_| gamma).transform_point3(Vec3::X);
    let by_body = Quat::from_rotation_z(r) * Vec3::X;
    assert!(
        by_bellows.distance(by_body) < 1e-4,
        "{by_bellows:?} vs {by_body:?}"
    );
}
