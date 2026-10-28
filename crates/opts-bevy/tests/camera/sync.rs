//! Integration tests for auxiliary mirror camera isolation and cockpit frame-sync.

use super::*;

/// A second `Camera3d` (the reflection prototype's mirror camera) must neither stop the camera
/// systems (`single()` queries) nor be moved by them: only the `MainCamera` follows the modes.
#[test]
fn camera_modes_ignore_an_extra_mirror_camera() {
    let mut app = app();
    let mirror_pos = Vec3::new(-500.0, 7.0, 400.0);
    let mirror = app
        .world_mut()
        .spawn((
            Camera3d::default(),
            Name::new("MirrorCamera"),
            Transform::from_translation(mirror_pos),
        ))
        .id();
    spawn_vehicle(&mut app);
    app.update();
    tap(&mut app, KeyCode::Digit2);
    assert_eq!(mode(&app), CameraMode::Cockpit);
    tap(&mut app, KeyCode::Digit5);
    assert_eq!(mode(&app), CameraMode::FreeRoam);
    // The free-roam spawn is the main camera's position, not the mirror's.
    assert_eq!(
        app.world().resource::<CameraState>().free_roam_spawn,
        Vec3::new(1.0, 2.0, 3.0)
    );
    app.update();
    assert_eq!(
        app.world().get::<Transform>(mirror).unwrap().translation,
        mirror_pos,
        "the mirror camera is never moved by the camera modes"
    );
}

/// The cockpit camera must be rendered where its `Transform` says, in the frame it was written:
/// the camera systems run after transform propagation, so without a re-sync its global
/// transform is one frame old and the view lags behind the moving cab by `speed x frame time`
/// (jitter in the driver view).
#[test]
fn the_driver_camera_does_not_lag_a_frame_behind_a_moving_vehicle() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::transform::TransformPlugin))
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ButtonInput<MouseButton>>()
        .add_plugins(opts_bevy::OptsCameraPlugin);
    spawn_vehicle(&mut app);
    let vehicle = {
        let mut q = app
            .world_mut()
            .query_filtered::<Entity, With<PlayerVehicle>>();
        q.single(app.world()).unwrap()
    };
    app.world_mut()
        .entity_mut(vehicle)
        .insert(Transform::default());
    let camera = app
        .world_mut()
        .spawn((
            Camera3d::default(),
            opts_bevy::MainCamera,
            Transform::default(),
        ))
        .id();
    app.update();
    tap(&mut app, KeyCode::Digit2);
    assert_eq!(mode(&app), CameraMode::Cockpit);

    // Irregular frame steps, as at a varying frame rate: the offset must stay constant.
    let mut x = 0.0_f32;
    let mut offset = None;
    for step in [0.1_f32, 0.35, 0.05, 0.6, 0.2, 0.9, 0.02] {
        x += step;
        app.world_mut()
            .entity_mut(vehicle)
            .get_mut::<Transform>()
            .unwrap()
            .translation
            .x = x;
        app.update();
        let v = app.world().get::<GlobalTransform>(vehicle).unwrap();
        let c = app.world().get::<GlobalTransform>(camera).unwrap();
        let t = app.world().get::<Transform>(camera).unwrap();
        assert!(
            (c.translation() - t.translation).length() < 1e-5,
            "the camera is drawn where its Transform is"
        );
        let rel = c.translation() - v.translation();
        match offset {
            None => offset = Some(rel),
            Some(o) => assert!((rel - o).length() < 1e-4, "{rel} vs {o}"),
        }
    }
}
