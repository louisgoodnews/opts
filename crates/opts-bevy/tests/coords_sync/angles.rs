//! Rotation mapping, euler round trips and the coordinate components.

use super::*;

#[test]
fn test_pitch_and_roll_mapping() {
    // Pitch +90 deg: nose up around +X. Forward (-Z) rotates to Up (+Y).
    let quat_pitch = omsi_euler_to_bevy_quat(0.0, 90.0, 0.0);
    let rotated = quat_pitch * Vec3::NEG_Z;
    assert!(
        (rotated - Vec3::Y).length() < 1e-5,
        "Pitch +90 deg failed: got {rotated:?}"
    );

    // Roll +90 deg: banking clockwise along vehicle axis (OMSI +Y / Bevy -Z).
    // In Bevy, with rotation around -Z, +Y (Up) rotates to +X (Right),
    // and +X (Right) rotates to -Y (Down).
    let quat_roll = omsi_euler_to_bevy_quat(0.0, 0.0, 90.0);
    let rotated_up = quat_roll * Vec3::Y;
    assert!(
        (rotated_up - Vec3::X).length() < 1e-5,
        "Roll +90 deg (Up -> Right) failed: got {rotated_up:?}"
    );

    let rotated_right = quat_roll * Vec3::X;
    assert!(
        (rotated_right - Vec3::NEG_Y).length() < 1e-5,
        "Roll +90 deg (Right -> Down) failed: got {rotated_right:?}"
    );
}

#[test]
fn test_euler_round_trip() {
    let test_angles = [
        (0.0, 0.0, 0.0),
        (45.0, 15.0, -10.0),
        (-30.0, -20.0, 15.0),
        (120.0, 0.0, 0.0),
        (0.0, 35.0, 0.0),
    ];

    for &(yaw, pitch, roll) in &test_angles {
        let quat = omsi_euler_to_bevy_quat(yaw, pitch, roll);
        let (r_yaw, r_pitch, r_roll) = bevy_quat_to_omsi_euler(quat);

        assert!(
            (yaw - r_yaw).abs() < 1e-3,
            "Yaw mismatch: expected {yaw}, got {r_yaw}"
        );
        assert!(
            (pitch - r_pitch).abs() < 1e-3,
            "Pitch mismatch: expected {pitch}, got {r_pitch}"
        );
        assert!(
            (roll - r_roll).abs() < 1e-3,
            "Roll mismatch: expected {roll}, got {r_roll}"
        );
    }
}

#[test]
fn test_omsi_coords_component() {
    let coords = OmsiCoords::new(3.0, 12.0, 1.5);
    let bevy_v = coords.to_bevy();
    assert_eq!(bevy_v, Vec3::new(3.0, 1.5, -12.0));

    let from_bevy = OmsiCoords::from_bevy(bevy_v);
    assert_eq!(from_bevy, coords);
}

#[test]
fn test_omsi_transform_helpers() {
    let omsi_tx =
        OmsiTransform::from_translation_rotation(Vec3::new(1.0, 2.0, 3.0), 90.0, 0.0, 0.0);

    let bevy_tx = omsi_tx.to_bevy_transform();
    assert_eq!(bevy_tx.translation, Vec3::new(1.0, 3.0, -2.0));

    let back = OmsiTransform::from_bevy_transform(&bevy_tx);
    assert!((back.translation - omsi_tx.translation).length() < 1e-5);
    assert!((back.rotation_deg.x - omsi_tx.rotation_deg.x).abs() < 1e-3);
}
