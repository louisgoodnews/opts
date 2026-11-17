//! Tests for `super` (`trailer_front.rs`): the joint against the render's own rotation.

use super::*;
use crate::vehicle::trailer::TrailerKinematics;
use glam::{DVec2, DVec4};

/// The hitch of the Kajosoft Citybus 530 Facelift in the leader's OMSI frame.
const H: DVec3 = DVec3::new(0.0, -4.421, 0.4);

/// Fixture coupling offset of the trailer: front coupling 1 m ahead of its origin.
const FRONT_LOCAL: DVec2 = DVec2::new(0.0, 1.0);

/// Points on the trailer's front face at rest, in the leader's OMSI frame.
fn front_face() -> [DVec3; 4] {
    [
        H + DVec3::new(-1.2, 0.0, 0.0),
        H + DVec3::new(1.2, 0.2, 1.5),
        H + DVec3::new(-0.6, 0.0, 1.8),
        H + DVec3::new(0.6, 0.2, 0.0),
    ]
}

/// The render's Bevy mapping of an OMSI point (`opts-bevy/src/coords/convert.rs`).
fn to_bevy(p: DVec3) -> DVec3 {
    DVec3::new(p.x, p.z, -p.y)
}

fn from_bevy(p: DVec3) -> DVec3 {
    DVec3::new(p.x, -p.z, p.y)
}

/// The render's rigid transform of a rest point, in Bevy coordinates, as
/// `articulation_render_system` builds it: `Quat::from_rotation_y(-h) * Rx(pitch) * Rz(roll)` with
/// the relative `pitch = -β`, `roll = γ` and `h = α` (the segment's heading against the leader),
/// pivoting about the hitch. Only the rotation builder is shared with the joint's sign convention;
/// the mapping and the composition are written out here so the test checks it independently.
fn render_point(p: DVec3, alpha: f64, beta: f64, gamma: f64) -> DVec3 {
    let q = DQuat::from_rotation_y(-alpha)
        * DQuat::from_rotation_x(-beta)
        * DQuat::from_rotation_z(gamma);
    let hb = to_bevy(H);
    from_bevy(hb + q * (to_bevy(p) - hb))
}

fn assert_matches_render(alpha: f64, beta: f64, gamma: f64) {
    let j = joint_transform(H, alpha, beta, gamma);
    for p in front_face() {
        let got = (j * DVec4::from((p, 1.0))).truncate();
        let want = render_point(p, alpha, beta, gamma);
        assert!(
            (got - want).length() < 1e-4,
            "alpha {alpha} beta {beta} gamma {gamma}: {got:?} vs {want:?}"
        );
    }
}

#[test]
fn identity_at_rest() {
    let j = joint_transform(H, 0.0, 0.0, 0.0);
    assert_eq!(j.to_cols_array(), DMat4::IDENTITY.to_cols_array());
}

#[test]
fn yaw_matches_render_and_the_segment_origin() {
    for deg in [12.0_f64, 25.0, -12.0, -25.0] {
        let alpha = deg.to_radians();
        assert_matches_render(alpha, 0.0, 0.0);
        // The segment origin the simulation places: `reseed` with the heading `alpha` and a
        // hitch at `H` (no hitch offset in the leader's frame).
        let mut seg = TrailerKinematics::new(
            DVec2::ZERO,
            FRONT_LOCAL,
            8.0,
            60.0,
            DVec2::new(H.x, H.y),
            0.0,
        );
        seg.reseed(DVec2::new(H.x, H.y), alpha);
        assert!((seg.alpha_against(0.0) - alpha).abs() < 1e-12);
        let rest_origin = DVec3::new(H.x - FRONT_LOCAL.x, H.y - FRONT_LOCAL.y, H.z);
        let got =
            (joint_transform(H, alpha, 0.0, 0.0) * DVec4::from((rest_origin, 1.0))).truncate();
        let want = DVec3::new(seg.origin.x, seg.origin.y, H.z);
        assert!(
            (got - want).length() < 1e-4,
            "{deg} deg: {got:?} vs {want:?}"
        );
    }
}

#[test]
fn pitch_and_roll_match_render() {
    let (beta, gamma) = (3.0_f64.to_radians(), (-2.0_f64).to_radians());
    for deg in [0.0_f64, 12.0, -25.0] {
        assert_matches_render(deg.to_radians(), beta, gamma);
    }
}

#[test]
fn the_hitch_is_fixed() {
    for (a, b, g) in [(0.4, 0.05, -0.03), (-0.7, -0.02, 0.04), (0.0, 0.1, 0.1)] {
        let fixed = (joint_transform(H, a, b, g) * DVec4::from((H, 1.0))).truncate();
        assert!((fixed - H).length() < 1e-9, "{fixed:?}");
    }
}
