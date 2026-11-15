//! The shift and space keys: the external view reset and what each view keeps.

use super::*;

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
    assert_eq!(
        (s.external_yaw, s.external_zoom, s.pan_offset),
        (0.0, 14.0, Vec3::ZERO)
    );
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
    assert_eq!(
        (s.sub_camera_index, s.view_yaw, s.view_pitch),
        (0, 0.0, 0.0)
    );
    let vp = app.world().resource::<opts_bevy::ViewpointState>();
    assert!(!vp.editing);
    assert_eq!(
        (vp.active.x, vp.active.y),
        (0.0, 0.05),
        "the draft goes, the saved viewpoint stays"
    );
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
    assert_eq!(
        (s.mode, s.view_yaw, s.view_pitch),
        (CameraMode::Passenger, 0.0, 0.0)
    );
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
