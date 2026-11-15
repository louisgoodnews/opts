//! The system: the main camera joins and leaves the cabin layer, and is published.

use super::*;

#[test]
fn the_main_camera_joins_and_leaves_the_cabin_layer_by_distance() {
    let mut app = app();
    // 30 m from the footprint's side.
    let cam = camera(&mut app, Vec3::new(31.3, 1.7, 0.0));
    app.update();
    app.update();
    assert_eq!(layers(&app, cam), camera_layers(false));
    assert!(!app.world().resource::<CabinView>().cabin_layer);
    move_camera(&mut app, cam, Vec3::new(1.3 + 13.0, 1.7, 0.0));
    assert_eq!(layers(&app, cam), camera_layers(false), "13 m: not yet");
    move_camera(&mut app, cam, Vec3::new(1.3 + 11.0, 1.7, 0.0));
    assert_eq!(layers(&app, cam), camera_layers(true), "11 m: in");
    move_camera(&mut app, cam, Vec3::new(1.3 + 14.0, 1.7, 0.0));
    assert_eq!(layers(&app, cam), camera_layers(true), "14 m: still in");
    assert!(app.world().resource::<CabinView>().cabin_layer);
    move_camera(&mut app, cam, Vec3::new(1.3 + 16.0, 1.7, 0.0));
    assert_eq!(layers(&app, cam), camera_layers(false), "16 m: out");
}

#[test]
fn an_interior_view_includes_the_cabin_layer_and_is_published() {
    let mut app = app();
    let cam = camera(&mut app, Vec3::new(60.0, 1.7, 0.0));
    app.insert_resource(crate::camera::CameraState {
        mode: crate::camera::CameraMode::Cockpit,
        ..default()
    });
    app.update();
    app.update();
    assert_eq!(layers(&app, cam), camera_layers(true));
    let view = *app.world().resource::<CabinView>();
    assert!(view.interior_view && view.cabin_layer);
    // Back to the external view: out again.
    app.insert_resource(crate::camera::CameraState {
        mode: crate::camera::CameraMode::External,
        ..default()
    });
    app.update();
    app.update();
    assert_eq!(layers(&app, cam), camera_layers(false));
    assert!(!app.world().resource::<CabinView>().interior_view);
}

#[test]
fn without_a_player_vehicle_the_camera_stays_on_the_world_layer() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::transform::TransformPlugin))
        .add_plugins(LightRoomsPlugin);
    let cam = camera(&mut app, Vec3::ZERO);
    app.update();
    app.update();
    assert_eq!(layers(&app, cam), camera_layers(false));
}
