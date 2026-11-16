//! Camera look, move, viewpoint and reset.

use super::*;

#[test]
fn camera_look_and_move_follow_the_mode_and_the_gates() {
    let mut app = drive_app();
    // Not in free roam: absolute angles and movement are refused with a reason.
    let e = call(&mut app, "camera.look", json!({"yaw": 45.0})).expect_err("yaw outside free roam");
    assert_eq!(e.code, code::NOT_READY);
    let e =
        call(&mut app, "camera.move", json!({"forward": 1.0})).expect_err("move outside free roam");
    assert_eq!(e.code, code::NOT_READY);
    let e = call(&mut app, "camera.look", json!({})).expect_err("nothing to do");
    assert_eq!(e.code, code::INVALID_PARAMS);
    // Free roam: yaw and pitch set the view directly (compass degrees, like `camera.teleport`).
    app.world_mut().resource_mut::<CameraState>().mode = CameraMode::FreeRoam;
    let look = call(
        &mut app,
        "camera.look",
        json!({"yaw": 90.0, "pitch": -10.0}),
    )
    .expect("look");
    assert!((look["yaw_deg"].as_f64().unwrap() - 90.0).abs() < 1e-3);
    assert!(
        (app.world().resource::<CameraState>().free_roam_pitch + 10f32.to_radians()).abs() < 1e-5
    );
    // Movement holds the movement keys and schedules their release.
    call(
        &mut app,
        "camera.move",
        json!({"forward": 1.0, "right": -1.0, "duration_ms": 200}),
    )
    .expect("move");
    let releases = app
        .world()
        .resource::<opts_bevy::control::state::ControlState>()
        .releases
        .len();
    assert_eq!(releases, 2, "W and A are released later");
    // A dialog blocks both, like the camera keys.
    app.insert_resource(ModalActive(true));
    let e = call(&mut app, "camera.move", json!({"forward": 1.0})).expect_err("gated");
    assert_eq!(e.code, code::GATED);
    let e = call(&mut app, "camera.look", json!({"dx": 5.0})).expect_err("gated");
    assert_eq!(e.code, code::GATED);
}

mod viewpoint;
