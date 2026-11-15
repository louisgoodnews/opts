//! Mini-map tests: centring without a vehicle, switching the map off and north-up.

use super::*;

#[test]
fn without_a_vehicle_the_map_is_centred_on_the_free_camera() {
    let mut app = map_app();
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<opts_bevy::PlayerVehicle>>();
    let player = q.iter(app.world()).next().expect("player");
    app.world_mut().despawn(player);
    app.world_mut().spawn((
        Camera3d::default(),
        opts_bevy::MainCamera,
        GlobalTransform::from(Transform::from_xyz(100.0, 5.0, -50.0)),
    ));
    // An auxiliary camera (a reflection mirror) is not the player's view.
    app.world_mut().spawn((
        Camera3d::default(),
        Name::new("MirrorCamera"),
        GlobalTransform::from(Transform::from_xyz(-900.0, 5.0, 700.0)),
    ));
    run(&mut app, 30);
    let s = minimap_state(&app);
    let c = s.centre.expect("centre");
    assert!(
        (c.x - 100.0).abs() < 1e-3 && (c.y - 50.0).abs() < 1e-3,
        "{c:?}"
    );
    // The camera is a marker, and so is the other session vehicle.
    assert_eq!(s.markers, 2);
}

#[test]
fn the_map_can_be_switched_off_and_the_widget_leaves_the_layout() {
    let mut app = map_app();
    run(&mut app, 30);
    assert_eq!(display_of(&mut app, "minimap"), Some(Display::Flex));
    app.world_mut()
        .resource_mut::<GameSettings>()
        .set_raw(SETTING_ENABLED, "false");
    run(&mut app, 3);
    assert_eq!(display_of(&mut app, "minimap"), Some(Display::None));
    let frames = minimap_state(&app).frames;
    run(&mut app, 60);
    assert_eq!(
        minimap_state(&app).frames,
        frames,
        "nothing is drawn while it is off"
    );
    // The slot is removed from the layout too.
    let slot = app
        .world_mut()
        .query::<(&build::HudSlot, &Node)>()
        .iter(app.world())
        .find(|(s, _)| s.0 == Slot::MiniMap)
        .map(|(_, n)| n.display);
    assert_eq!(slot, Some(Display::None));
}

#[test]
fn north_up_keeps_north_up() {
    let mut app = map_app();
    app.world_mut()
        .resource_mut::<GameSettings>()
        .set_raw(SETTING_ORIENTATION, "north");
    run(&mut app, 30);
    assert_eq!(minimap_state(&app).up_heading_deg, 0.0);
}
