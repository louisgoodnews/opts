//! Integration tests for the camera perspective system.

use bevy::input::ButtonInput;
use bevy::prelude::*;
use opts_bevy::camera::input::{
    handle_camera_key_input, handle_camera_reset_input, update_camera_availability,
};
use opts_bevy::vehicle::BusVehicle;
use opts_bevy::{
    scale_free_roam_speed, CameraAvailability, CameraMode, CameraState, PlayerVehicle,
    FREE_ROAM_MIN_SPEED,
};

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<CameraState>()
        .init_resource::<CameraAvailability>()
        .init_resource::<opts_bevy::ViewpointState>()
        .add_systems(
            Update,
            (
                update_camera_availability,
                handle_camera_key_input,
                handle_camera_reset_input,
            )
                .chain(),
        );
    app.world_mut()
        .spawn((Camera3d::default(), Transform::from_xyz(1.0, 2.0, 3.0)));
    app
}

fn spawn_vehicle(app: &mut App) {
    let cam = opts_core::formats::vehicles::BusCamera {
        x: 0.0,
        y: 0.0,
        z: 1.0,
        dist: 0.0,
        fov_deg: 90.0,
        yaw_deg: 0.0,
        pitch_deg: 0.0,
        is_driver: true,
        is_schedule_view: false,
        is_ticket_view: false,
    };
    app.world_mut().spawn((
        PlayerVehicle,
        BusVehicle {
            friendly_name: "Test".into(),
            mass_tonnes: None,
            driver_cameras: vec![cam.clone(), cam.clone()],
            pax_cameras: vec![cam],
            trailer_pax_cameras: Vec::new(),
            outside_camera_center: None,
            std_camera_index: 1,
            active_camera_index: 0,
        },
    ));
}

fn tap(app: &mut App, key: KeyCode) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(key);
    app.update();
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.release(key);
    keys.clear();
}

fn mode(app: &App) -> CameraMode {
    app.world().resource::<CameraState>().mode
}

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
        .query_filtered::<&mut Transform, With<Camera3d>>();
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

fn shift_space(app: &mut App) {
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::ShiftLeft);
        keys.press(KeyCode::Space);
    }
    app.update();
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.release(KeyCode::ShiftLeft);
    keys.release(KeyCode::Space);
    keys.clear();
}

#[test]
fn shift_space_resets_the_external_view_to_its_defaults() {
    let mut app = app();
    spawn_vehicle(&mut app);
    app.update();
    tap(&mut app, KeyCode::Digit3);
    {
        let mut s = app.world_mut().resource_mut::<CameraState>();
        s.external_yaw = 2.0;
        s.external_pitch = 1.0;
        s.external_zoom = 40.0;
        s.pan_offset = Vec3::ONE;
    }
    shift_space(&mut app);
    let s = app.world().resource::<CameraState>();
    assert_eq!((s.external_yaw, s.external_zoom, s.pan_offset), (0.0, 8.0, Vec3::ZERO));
    assert!((s.external_pitch + 15f32.to_radians()).abs() < 1e-6);
    assert_eq!(s.mode, CameraMode::External);
}

#[test]
fn shift_space_in_the_driver_view_keeps_the_sub_camera_and_the_stored_viewpoint() {
    let mut app = app();
    spawn_vehicle(&mut app);
    app.update();
    tap(&mut app, KeyCode::Digit2);
    {
        let mut s = app.world_mut().resource_mut::<CameraState>();
        s.sub_camera_index = 0; // not the standard camera
        s.view_yaw = 1.0;
        s.view_pitch = -0.4;
    }
    {
        let mut vp = app.world_mut().resource_mut::<opts_bevy::ViewpointState>();
        vp.committed.y = 0.05;
        vp.active.y = 0.05;
        vp.open();
        vp.adjust(opts_bevy::ViewpointRow::X, 20.0);
    }
    shift_space(&mut app);
    let s = app.world().resource::<CameraState>();
    assert_eq!((s.sub_camera_index, s.view_yaw, s.view_pitch), (0, 0.0, 0.0));
    let vp = app.world().resource::<opts_bevy::ViewpointState>();
    assert!(!vp.editing);
    assert_eq!((vp.active.x, vp.active.y), (0.0, 0.05), "the draft goes, the saved viewpoint stays");
}

#[test]
fn shift_space_in_the_passenger_view_keeps_the_sub_camera() {
    let mut app = app();
    spawn_vehicle(&mut app);
    app.update();
    tap(&mut app, KeyCode::Digit4);
    {
        let mut s = app.world_mut().resource_mut::<CameraState>();
        s.view_yaw = 1.5;
        s.view_pitch = 0.3;
    }
    shift_space(&mut app);
    let s = app.world().resource::<CameraState>();
    assert_eq!((s.mode, s.view_yaw, s.view_pitch), (CameraMode::Passenger, 0.0, 0.0));
}

#[test]
fn shift_space_in_the_walking_view_looks_forward_again() {
    let mut app = app();
    app.update();
    {
        let mut s = app.world_mut().resource_mut::<CameraState>();
        s.mode = CameraMode::Ego;
        s.ego_yaw = 1.0;
        s.ego_pitch = 0.5;
    }
    shift_space(&mut app);
    let s = app.world().resource::<CameraState>();
    assert_eq!((s.ego_yaw, s.ego_pitch), (0.0, 0.0));
}
