//! Integration tests for coordinate frame conversion functions, components,
//! and Bevy ECS synchronization systems.

use bevy::app::App;
use bevy::math::Vec3;
use bevy::transform::components::Transform;
use bevy::MinimalPlugins;

use opts_bevy::{
    bevy_quat_to_omsi_euler, bevy_to_omsi_pos, omsi_euler_to_bevy_quat, omsi_to_bevy_pos,
    OmsiCoords, OmsiTransform, OptsCoordsPlugin,
};

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

#[test]
fn test_headless_bevy_sync_systems() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(OptsCoordsPlugin);

    // Spawn an entity with OmsiTransform and default Transform
    let entity = app
        .world_mut()
        .spawn((
            OmsiTransform::from_translation(Vec3::new(1.0, 5.0, 2.0)),
            Transform::default(),
        ))
        .id();

    // First update: OmsiTransform should sync to Transform
    app.update();

    let transform = *app
        .world()
        .entity(entity)
        .get::<Transform>()
        .expect("Transform exists");
    assert_eq!(transform.translation, Vec3::new(1.0, 2.0, -5.0));

    // Mutate OmsiTransform: change translation
    {
        let mut entity_ref = app.world_mut().entity_mut(entity);
        let mut omsi_tx = entity_ref
            .get_mut::<OmsiTransform>()
            .expect("OmsiTransform exists");
        omsi_tx.translation = Vec3::new(3.0, 8.0, -1.0);
    }

    app.update();

    let updated_tx = *app
        .world()
        .entity(entity)
        .get::<Transform>()
        .expect("Transform exists");
    assert_eq!(updated_tx.translation, Vec3::new(3.0, -1.0, -8.0));

    // Reverse sync: Mutate Bevy Transform directly
    {
        let mut entity_ref = app.world_mut().entity_mut(entity);
        let mut bevy_tx = entity_ref.get_mut::<Transform>().expect("Transform exists");
        bevy_tx.translation = Vec3::new(5.0, 10.0, -15.0);
    }

    app.update();

    let back_omsi_tx = *app
        .world()
        .entity(entity)
        .get::<OmsiTransform>()
        .expect("OmsiTransform exists");
    assert_eq!(back_omsi_tx.translation, Vec3::new(5.0, 15.0, 10.0));

    // Stability test: Run multiple updates to ensure no runaway feedback loop
    for _ in 0..5 {
        app.update();
    }

    let stable_bevy = *app
        .world()
        .entity(entity)
        .get::<Transform>()
        .expect("Transform exists");
    let stable_omsi = *app
        .world()
        .entity(entity)
        .get::<OmsiTransform>()
        .expect("OmsiTransform exists");
    assert_eq!(stable_bevy.translation, Vec3::new(5.0, 10.0, -15.0));
    assert_eq!(stable_omsi.translation, Vec3::new(5.0, 15.0, 10.0));
}
