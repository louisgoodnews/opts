//! Tests of the door camera: the doors of a cabin, the pose, the choice of the door.

use super::*;

/// Right front (entry), right front behind it (entry), right rear (exit), left rear (exit), in
/// Bevy axes (forward is -z).
fn doors() -> Vec<DoorPoint> {
    let door = |entry, index, x: f32, z: f32| DoorPoint {
        entry,
        index,
        position: Vec3::new(x, 0.4, z),
        side: if x < 0.0 { -1 } else { 1 },
    };
    vec![
        door(true, 3, 1.25, -4.9),
        door(true, 0, 1.25, -4.2),
        door(false, 23, 1.25, 2.4),
        door(false, 21, -1.2, 2.5),
    ]
}

#[test]
fn zones_follow_the_position_along_the_vehicle() {
    let points = DoorPoints(doors());
    // z: -4.9, -4.2 (front), -2.4 and -2.5 are at the rear end of this door set.
    assert_eq!(points.zone(0), Zone::Front);
    assert_eq!(points.zone(1), Zone::Front);
    assert_eq!(points.zone(2), Zone::Rear);
    assert_eq!(points.zone(3), Zone::Rear);
    // Doors within a metre of each other have no front or rear.
    let near = DoorPoints(vec![doors()[0].clone(), doors()[1].clone()]);
    assert_eq!(near.zone(0), Zone::Middle);
    assert_eq!(points.zone(99), Zone::Middle);
}

#[test]
fn only_door_leaf_variables_count() {
    for yes in ["door_01", "c2_tuer_1_v_pos", "Door_2", "tür_pos"] {
        assert!(is_door_variable(yes), "{yes}");
    }
    for no in [
        "door_01_alpha",
        "door_warn",
        "doorspeed_0",
        "c2_cp_bt_tuer_1",
        "doortriggeropen_0",
        "c2_tuer_1_sound",
        "c2_tuer_2_open_req",
        "c2_tuer_1_h_stopped",
        "speed",
        "",
    ] {
        assert!(!is_door_variable(no), "{no}");
    }
}

#[test]
fn the_nearest_door_to_a_leaf_is_found_in_the_floor_plane() {
    let d = doors();
    // A leaf at the right front (Bevy axes: forward is -z), a little above the floor and outside.
    assert_eq!(nearest_door(&d, Vec3::new(1.5, 1.2, -4.7)), Some(0));
    assert_eq!(nearest_door(&d, Vec3::new(1.5, 1.2, -4.25)), Some(1));
    // A leaf at the left rear.
    assert_eq!(nearest_door(&d, Vec3::new(-1.3, 1.0, 2.45)), Some(3));
    assert_eq!(nearest_door(&[], Vec3::ZERO), None);
}

#[test]
fn f4_cycles_through_the_doors_and_back_to_none_and_the_setting_gates_everything() {
    let mut s = DoorCamera::default();
    assert_eq!(s.shown(), None, "off by default");
    s.manual = Some(1);
    assert_eq!(
        s.shown(),
        None,
        "the setting is off: nothing is shown whatever was chosen"
    );
    s.enabled = true;
    s.manual = None;
    for want in [Some(0), Some(1), Some(2), None, Some(0)] {
        s.cycle(3);
        assert_eq!(s.manual, want);
    }
    s.manual = None;
    s.cycle(0);
    assert_eq!(s.manual, None, "no doors, nothing to choose");
    // The manual choice wins over the automatic one.
    s.auto = Some(2);
    assert_eq!(s.shown(), Some(2));
    s.manual = Some(1);
    assert_eq!(s.shown(), Some(1));
}

mod detection {
    use super::*;
    use crate::vehicle_anim::AnimStep;
    use std::time::Duration;

    fn step(variable: &str) -> AnimStep {
        AnimStep {
            frame: Mat4::IDENTITY,
            variable: variable.into(),
            factor: 1.0,
            translate: false,
            maxspeed: None,
        }
    }

    /// A vehicle at (100, 0, 50) turned half way round, with the door points of `doors()`.
    fn app() -> (App, Entity, Transform) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                Duration::from_millis(300),
            ))
            .insert_resource(DoorCamera {
                enabled: true,
                ..default()
            })
            .add_systems(Update, detect_open_doors);
        let at = Transform::from_xyz(100.0, 0.0, 50.0)
            .with_rotation(Quat::from_rotation_y(std::f32::consts::PI));
        let vehicle = app
            .world_mut()
            .spawn((
                PlayerVehicle,
                DoorPoints(doors()),
                GlobalTransform::from(at),
            ))
            .id();
        (app, vehicle, at)
    }

    fn leaf(
        app: &mut App,
        vehicle: Entity,
        at: Transform,
        local: Vec3,
        variable: &str,
        value: f32,
    ) -> Entity {
        app.world_mut()
            .spawn((
                MeshAnimation {
                    owner: vehicle,
                    steps: vec![AnimStep {
                        // The pivot in vehicle axes (x right, y forward, z up).
                        frame: Mat4::from_translation(Vec3::new(local.x, -local.z, local.y)),
                        ..step(variable)
                    }],
                    visible: Vec::new(),
                    shown: vec![value],
                },
                GlobalTransform::from(Transform::from_translation(at.transform_point(local))),
            ))
            .id()
    }

    fn run(app: &mut App, frames: usize) {
        for _ in 0..frames {
            app.update();
        }
    }

    #[test]
    fn an_opening_leaf_shows_the_nearest_door_and_it_is_held_for_a_moment_after() {
        let (mut app, vehicle, at) = app();
        // Not a door variable, and a door variable at rest: nothing.
        leaf(
            &mut app,
            vehicle,
            at,
            Vec3::new(1.4, 1.0, -4.8),
            "wiper_pos",
            1.0,
        );
        leaf(
            &mut app,
            vehicle,
            at,
            Vec3::new(1.4, 1.0, -4.8),
            "c2_tuer_1_v_pos",
            0.0,
        );
        run(&mut app, 3);
        assert_eq!(app.world().resource::<DoorCamera>().auto, None);
        // The leaf at the left rear door opens (the vehicle is turned: world positions differ).
        let open = leaf(
            &mut app,
            vehicle,
            at,
            Vec3::new(-1.3, 1.0, 2.45),
            "c2_tuer_4_h_pos",
            0.7,
        );
        run(&mut app, 2);
        assert_eq!(app.world().resource::<DoorCamera>().auto, Some(3));
        // The leaf rests: the door stays for HOLD_S (2 s = about 7 frames of 0.3 s), then goes.
        app.world_mut()
            .get_mut::<MeshAnimation>(open)
            .expect("anim")
            .shown = vec![0.0];
        run(&mut app, 3);
        assert_eq!(app.world().resource::<DoorCamera>().auto, Some(3), "held");
        run(&mut app, 12);
        assert_eq!(app.world().resource::<DoorCamera>().auto, None, "released");
    }

    #[test]
    fn nothing_is_looked_at_while_the_setting_is_off_or_for_another_vehicles_leaves() {
        let (mut app, vehicle, at) = app();
        let other = app.world_mut().spawn_empty().id();
        leaf(
            &mut app,
            other,
            at,
            Vec3::new(1.4, 1.0, -4.8),
            "door_01",
            1.0,
        );
        run(&mut app, 3);
        assert_eq!(
            app.world().resource::<DoorCamera>().auto,
            None,
            "not this vehicle's leaf"
        );
        leaf(
            &mut app,
            vehicle,
            at,
            Vec3::new(1.4, 1.0, -4.8),
            "door_01",
            1.0,
        );
        app.world_mut().resource_mut::<DoorCamera>().enabled = false;
        run(&mut app, 3);
        assert_eq!(
            app.world().resource::<DoorCamera>().auto,
            None,
            "setting off"
        );
        app.world_mut().resource_mut::<DoorCamera>().enabled = true;
        run(&mut app, 3);
        assert_eq!(app.world().resource::<DoorCamera>().auto, Some(0));
    }
}

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

#[test]
fn the_camera_hangs_at_the_upper_rear_of_a_right_door_and_the_upper_front_of_a_left_one() {
    let right = DoorPoint {
        entry: true,
        index: 0,
        position: Vec3::new(1.2, 0.0, -3.0),
        side: 1,
    };
    let left = DoorPoint {
        side: -1,
        position: Vec3::new(-1.2, 0.0, -3.0),
        ..right.clone()
    };
    let r = door_pose(&right);
    let l = door_pose(&left);
    // Bevy: z backward. The right door's camera is behind it, the left one's in front of it.
    assert!(r.translation.z > right.position.z && l.translation.z < left.position.z);
    assert!(r.translation.x > right.position.x && l.translation.x < left.position.x);
    assert!(r.translation.y > 1.5, "above the door");
    // Both look at the doorway: the door point lies in front of the lens.
    for (pose, door) in [(r, &right), (l, &left)] {
        let to_door = (door.position + Vec3::Y * 0.7 - pose.translation).normalize();
        assert!(pose.forward().dot(to_door) > 0.99);
    }
}

#[test]
fn the_points_of_one_entrance_are_one_door() {
    let p = |z: f32, side: i8| DoorPoint {
        entry: true,
        index: 0,
        position: Vec3::new(f32::from(side), 0.0, z),
        side,
    };
    // Two lanes of the front door, two of the rear door, one on the left.
    let d = vec![p(-5.3, 1), p(-4.6, 1), p(-0.7, 1), p(-0.2, 1), p(-5.0, -1)];
    let groups: Vec<usize> = (0..d.len()).map(|i| door_group(&d, i)).collect();
    assert_eq!(groups, vec![0, 0, 2, 2, 4]);
}
