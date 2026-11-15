//! The camera modes: free roam, the unavailable modes, the spawn and reset, the speed floor and the cockpit start.

use super::*;

#[test]
fn free_roam_is_always_available_and_f1_toggles() {
    let mut app = app();
    spawn_vehicle(&mut app);
    app.update();
    tap(&mut app, KeyCode::Digit2);
    assert_eq!(mode(&app), CameraMode::Cockpit);
    tap(&mut app, KeyCode::Digit5);
    assert_eq!(mode(&app), CameraMode::FreeRoam);
    tap(&mut app, KeyCode::Digit0);
    assert_eq!(mode(&app), CameraMode::Cockpit);
    tap(&mut app, KeyCode::Digit0);
    assert_eq!(mode(&app), CameraMode::FreeRoam);
}

#[test]
fn unavailable_modes_do_not_change_state() {
    let mut app = app();
    for key in [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
    ] {
        tap(&mut app, key);
        assert_eq!(
            mode(&app),
            CameraMode::FreeRoam,
            "{key:?} without target must be ignored"
        );
    }
    assert_eq!(
        app.world().resource::<CameraState>().previous_mode,
        CameraMode::FreeRoam
    );
}

#[test]
fn entering_free_roam_records_spawn_and_reset_restores_it() {
    let mut app = app();
    spawn_vehicle(&mut app);
    app.update();
    tap(&mut app, KeyCode::Digit2);
    tap(&mut app, KeyCode::Digit5);
    let spawn = app.world().resource::<CameraState>().free_roam_spawn;
    assert_eq!(spawn, Vec3::new(1.0, 2.0, 3.0));

    // Fly away, speed up, then Shift+Space.
    {
        let mut state = app.world_mut().resource_mut::<CameraState>();
        state.free_roam_speed = 50.0;
        state.free_roam_yaw = 1.0;
    }
    let mut cams = app
        .world_mut()
        .query_filtered::<&mut Transform, With<opts_bevy::MainCamera>>();
    cams.single_mut(app.world_mut()).unwrap().translation = Vec3::splat(99.0);

    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::ShiftLeft);
        keys.press(KeyCode::Space);
    }
    app.update();

    let state = app.world().resource::<CameraState>();
    assert_eq!(state.free_roam_speed, FREE_ROAM_MIN_SPEED);
    assert_eq!(state.free_roam_yaw, 0.0);
    assert_eq!(
        state.mode,
        CameraMode::FreeRoam,
        "reset must not change the mode"
    );
    assert_eq!(
        state.previous_mode,
        CameraMode::Cockpit,
        "reset must not touch previous_mode"
    );
    let t = cams.single(app.world()).unwrap();
    assert_eq!(t.translation, spawn);
}

#[test]
fn speed_has_floor_but_no_ceiling() {
    assert_eq!(
        scale_free_roam_speed(FREE_ROAM_MIN_SPEED, -10.0),
        FREE_ROAM_MIN_SPEED
    );
    let mut s = FREE_ROAM_MIN_SPEED;
    for _ in 0..200 {
        s = scale_free_roam_speed(s, 1.0);
    }
    assert!(s > 1.0e15);
}

#[test]
fn cockpit_starts_at_the_vehicles_standard_camera_and_passenger_is_available() {
    let mut app = app();
    spawn_vehicle(&mut app);
    app.update();
    tap(&mut app, KeyCode::Digit2);
    assert_eq!(
        app.world().resource::<CameraState>().sub_camera_index,
        1,
        "[set_camera_std]"
    );
    tap(&mut app, KeyCode::Digit4);
    assert_eq!(mode(&app), CameraMode::Passenger);
    assert_eq!(app.world().resource::<CameraState>().sub_camera_index, 0);
}
