//! Unit tests for `super` (`sky.rs`).

use super::*;

/// With a second `Camera3d` (a reflection mirror) the dome still follows only the main camera.
#[test]
fn the_dome_follows_the_main_camera_not_a_mirror_camera() {
    let mut app = App::new();
    app.insert_resource(SessionEnvironment::default())
        .add_systems(Update, place_sky_dome);
    let main_pos = Vec3::new(10.0, 2.0, -30.0);
    app.world_mut().spawn((
        Camera3d::default(),
        opts_bevy::MainCamera,
        GlobalTransform::from_translation(main_pos),
    ));
    app.world_mut().spawn((
        Camera3d::default(),
        Name::new("MirrorCamera"),
        GlobalTransform::from_translation(Vec3::new(-500.0, 1.0, 400.0)),
    ));
    let dome = app.world_mut().spawn((SkyDome, Transform::default())).id();
    app.update();
    assert_eq!(
        app.world().get::<Transform>(dome).unwrap().translation,
        main_pos
    );
}
