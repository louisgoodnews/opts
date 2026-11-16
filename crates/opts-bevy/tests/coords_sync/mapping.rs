//! Position and axis mapping between OMSI and Bevy.

use super::*;

#[test]
fn test_exact_position_mapping() {
    // Acceptance criterion 2: (0, 10, 2) -> (0, 2, -10)
    let omsi_pos = Vec3::new(0.0, 10.0, 2.0);
    let bevy_pos = omsi_to_bevy_pos(omsi_pos);
    assert_eq!(bevy_pos, Vec3::new(0.0, 2.0, -10.0));
}

#[test]
fn test_cardinal_axes_mapping() {
    // OMSI Right (+X) -> Bevy Right (+X)
    assert_eq!(omsi_to_bevy_pos(Vec3::X), Vec3::X);

    // OMSI Forward (+Y) -> Bevy Forward (-Z)
    assert_eq!(omsi_to_bevy_pos(Vec3::Y), Vec3::NEG_Z);

    // OMSI Up (+Z) -> Bevy Up (+Y)
    assert_eq!(omsi_to_bevy_pos(Vec3::Z), Vec3::Y);

    // Inverse
    assert_eq!(bevy_to_omsi_pos(Vec3::X), Vec3::X);
    assert_eq!(bevy_to_omsi_pos(Vec3::NEG_Z), Vec3::Y);
    assert_eq!(bevy_to_omsi_pos(Vec3::Y), Vec3::Z);
}

#[test]
fn test_position_round_trip() {
    let test_vectors = [
        Vec3::ZERO,
        Vec3::new(1.0, 2.0, 3.0),
        Vec3::new(-10.5, 45.2, -3.5),
        Vec3::new(100.0, -50.0, 12.0),
        Vec3::new(0.001, 0.002, 0.003),
    ];

    for &v in &test_vectors {
        let bevy = omsi_to_bevy_pos(v);
        let back = bevy_to_omsi_pos(bevy);
        assert!(
            (back - v).length() < 1e-6,
            "Round trip failed for {v:?}: got {back:?}"
        );
    }
}

#[test]
fn test_yaw_rotation_mapping() {
    // In OMSI: forward is +Y, right is +X.
    // In Bevy: forward is -Z, right is +X, up is +Y.
    // A positive yaw of 90 degrees around +Z in OMSI is counter-clockwise looking from above.
    // Looking from above in Bevy (+Y), counter-clockwise rotation of forward (-Z) points to (-X) (left).
    let quat = omsi_euler_to_bevy_quat(90.0, 0.0, 0.0);
    let bevy_forward = Vec3::NEG_Z;
    let rotated_forward = quat * bevy_forward;

    assert!(
        (rotated_forward - Vec3::NEG_X).length() < 1e-5,
        "Expected yaw 90 deg to rotate forward to -X, got {rotated_forward:?}"
    );

    let bevy_right = Vec3::X;
    let rotated_right = quat * bevy_right;
    assert!(
        (rotated_right - Vec3::NEG_Z).length() < 1e-5,
        "Expected yaw 90 deg to rotate right to -Z, got {rotated_right:?}"
    );
}
