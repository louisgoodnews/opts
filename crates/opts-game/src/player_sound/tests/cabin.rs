//! Which voices the listener hears: the viewpoint rules, the cabin box and the camera mode.

use super::*;

#[test]
fn the_viewpoint_keeps_exterior_sounds_out_of_the_cabin() {
    // Two loops: rain for the outside only (`[viewpoint]` 5), a hum for everywhere. The camera at
    // (0, 1.5, 0) is inside the box of the vehicle at the origin, so only the hum may start.
    let cfg = "[sound]\nrain.wav\n1\n[3d]\n0\n0\n1\n10\n[viewpoint]\n5\n\
[sound]\nhum.wav\n1\n[3d]\n0\n0\n1\n10\n";
    let mut r = rig("view", cfg, "{frame}\n{end}");
    assert!(run(&mut r.app, |a| a
        .world()
        .resource::<AudioStats>()
        .emitters
        >= 1));
    run(&mut r.app, |_| false);
    assert!(r.app.world().resource::<PlayerAudioStats>().inside);
    assert_eq!(
        emitters(&mut r.app).len(),
        1,
        "only the hum is started inside"
    );
}

#[test]
fn the_cabin_box_is_the_body_of_a_bus() {
    let tf = GlobalTransform::from(Transform::from_xyz(10.0, 0.0, 0.0));
    assert!(listener_inside(&tf, Vec3::new(10.0, 1.5, 3.0)));
    assert!(!listener_inside(&tf, Vec3::new(14.0, 1.5, 0.0)));
    assert!(!listener_inside(&tf, Vec3::new(10.0, 6.0, 0.0)));
    assert!(!listener_inside(&tf, Vec3::new(10.0, 1.5, 12.0)));
}

#[test]
fn the_camera_mode_decides_the_room_of_a_bus() {
    use opts_bevy::CameraMode::{Cockpit, External, FreeRoam, Passenger};
    let tf = GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.0));
    // The external camera hangs a few metres behind the body: inside the body-sized box, but
    // outside the cabin.
    let behind = Vec3::new(0.0, 3.0, 7.0);
    assert!(listener_inside(&tf, behind));
    assert!(!listener_in_cabin(Some(External), &tf, behind));
    // The driver's and the passengers' views are in the cabin wherever the preset puts them.
    assert!(listener_in_cabin(Some(Cockpit), &tf, behind));
    assert!(listener_in_cabin(
        Some(Passenger),
        &tf,
        Vec3::new(40.0, 0.0, 0.0)
    ));
    // The free camera is where it is.
    assert!(listener_in_cabin(
        Some(FreeRoam),
        &tf,
        Vec3::new(0.0, 1.5, 0.0)
    ));
    assert!(!listener_in_cabin(
        Some(FreeRoam),
        &tf,
        Vec3::new(30.0, 1.5, 0.0)
    ));
    assert!(listener_in_cabin(None, &tf, Vec3::new(0.0, 1.5, 0.0)));
}
