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
    app.world_mut().spawn((
        Camera3d::default(),
        opts_bevy::MainCamera,
        Transform::from_xyz(1.0, 2.0, 3.0),
    ));
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
            trailer_driver_cameras: Vec::new(),
            mirror_cameras: Vec::new(),
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

#[path = "camera/modes.rs"]
mod modes;
#[path = "camera/shift.rs"]
mod shift;
#[path = "camera/drive.rs"]
mod drive;
#[path = "camera/sync.rs"]
mod sync;
