//! Tests of the door pictures: one slot per open door, fading after it closes, the rest moving up.

use super::*;
#[test]
fn every_open_door_has_a_picture_that_fades_after_it_closes_and_the_rest_move_up() {
    use std::time::Duration;
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .init_asset::<Image>()
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::from_secs(1),
        ))
        .insert_resource(DoorCamera {
            enabled: true,
            open: vec![0],
            ..default()
        })
        .add_systems(Update, drive_camera);
    app.world_mut().spawn((
        PlayerVehicle,
        DoorPoints(doors()),
        Transform::default(),
        GlobalTransform::default(),
    ));
    let cameras = |app: &mut App| {
        app.world_mut()
            .query::<&DoorCameraMarker>()
            .iter(app.world())
            .count()
    };
    let doors_of = |app: &App| -> Vec<usize> {
        app.world()
            .resource::<DoorCamera>()
            .slots
            .iter()
            .map(|s| s.door)
            .collect()
    };
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(doors_of(&app), vec![0]);
    app.world_mut().resource_mut::<DoorCamera>().open = vec![0, 1, 2, 3];
    for _ in 0..2 {
        app.update();
    }
    assert_eq!(doors_of(&app), vec![0, 1, 2, 3], "in the order they opened");
    app.world_mut().resource_mut::<DoorCamera>().open = vec![0, 1, 2, 3, 3];
    app.update();
    assert_eq!(cameras(&mut app), 4, "one camera per door, at most four");
    // Door 0 closes: two seconds untouched, then three of fading, then gone.
    app.world_mut().resource_mut::<DoorCamera>().open = vec![1, 2, 3];
    app.update();
    for _ in 0..2 {
        app.update();
    }
    let fade = app.world().resource::<DoorCamera>().slots[0].fade;
    assert!(fade > 0.99, "held for two seconds: {fade}");
    app.update();
    app.update();
    let fade = app.world().resource::<DoorCamera>().slots[0].fade;
    assert!(fade > 0.2 && fade < 0.9, "fading: {fade}");
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(doors_of(&app), vec![1, 2, 3], "gone, the others move up");
    assert_eq!(cameras(&mut app), 3);
    // It opens again: a new picture at the end.
    app.world_mut().resource_mut::<DoorCamera>().open = vec![1, 2, 3, 0];
    app.update();
    assert_eq!(doors_of(&app), vec![1, 2, 3, 0]);
}
