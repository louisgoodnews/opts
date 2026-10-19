//! Tests of the door camera: the doors of a cabin, the pose, the choice of the door.

use super::*;

const CABIN: &str = "[entry]\n3\n\n[entry]\n0\n\n[exit]\n23\n\n[exit]\n21\n\n[exit]\n99\n";

/// A paths file with `n` points: point `i` at x = `xs[i]`, y = `ys[i]`, z 0.4.
fn paths(points: &[(f32, f32)]) -> PassengerPaths {
    let mut text = String::new();
    for (x, y) in points {
        text.push_str(&format!("[pathpnt]\n{x}\n{y}\n0.4\n\n"));
    }
    PassengerPaths::parse(&text).expect("paths")
}

fn doors() -> Vec<DoorPoint> {
    // 24 points, only 0, 3, 21 and 23 matter: right-front, right-front-behind, left-rear, right-rear.
    let mut pts = vec![(0.0, 0.0); 24];
    pts[0] = (1.25, 4.2);
    pts[3] = (1.25, 4.9);
    pts[21] = (-1.2, -2.5);
    pts[23] = (1.25, -2.4);
    door_points(CABIN, &paths(&pts))
}

#[test]
fn the_doors_are_the_entries_then_the_exits_that_have_a_pathpoint() {
    let d = doors();
    let order: Vec<(bool, usize)> = d.iter().map(|p| (p.entry, p.index)).collect();
    assert_eq!(
        order,
        [(true, 3), (true, 0), (false, 23), (false, 21)],
        "99 has no point"
    );
    // Vehicle space (x right, y forward, z up) became Bevy axes (x right, y up, -z forward).
    assert_eq!(d[0].position, Vec3::new(1.25, 0.4, -4.9));
    assert_eq!((d[0].side, d[3].side), (1, -1));
}

#[test]
fn a_bad_file_gives_no_doors() {
    assert!(door_points_from_bytes(b"[entry]\n0\n", b"[pathpnt]\nx\ny\nz\n").is_empty());
    assert!(door_points_from_bytes(b"", b"").is_empty());
}

#[test]
fn zones_follow_the_position_along_the_vehicle() {
    let points = DoorPoints(doors());
    // z: -4.9, -4.2 (front), -2.4 and -2.5 are at the rear end of this door set.
    assert_eq!(points.zone(0), Zone::Front);
    assert_eq!(points.zone(1), Zone::Front);
    assert_eq!(points.zone(2), Zone::Rear);
    // Doors within a metre of each other have no front or rear.
    let near = DoorPoints(vec![doors()[0].clone(), doors()[1].clone()]);
    assert_eq!(near.zone(0), Zone::Middle);
    assert_eq!(points.zone(99), Zone::Middle);
}

#[test]
fn the_camera_sits_above_and_outside_the_door_and_looks_out_and_back() {
    let d = doors();
    let right = door_pose(&d[0]);
    assert!(
        (right.translation - (d[0].position + Vec3::new(EYE_OUT_M, EYE_HEIGHT_M, 0.0))).length()
            < 1e-5
    );
    let f = right.forward();
    assert!(f.x > 0.7, "looks out to the right: {f:?}");
    assert!(f.z > 0.2, "and back (towards +z, the rear): {f:?}");
    assert!(f.y < 0.0, "and a little down: {f:?}");
    let left = door_pose(&d[3]);
    assert!(left.forward().x < -0.7, "a left door looks out to the left");
    assert!(left.translation.x < d[3].position.x, "and sits outside it");
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
                    steps: vec![step(variable)],
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
fn the_camera_is_removed_after_five_quiet_seconds_and_made_again_when_a_door_is_shown() {
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
            manual: Some(0),
            ..default()
        })
        .add_systems(Update, drive_camera);
    let _vehicle = app
        .world_mut()
        .spawn((
            PlayerVehicle,
            DoorPoints(doors()),
            Transform::default(),
            GlobalTransform::default(),
        ))
        .id();
    let cameras = |app: &mut App| {
        app.world_mut()
            .query::<&DoorCameraMarker>()
            .iter(app.world())
            .count()
    };
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(cameras(&mut app), 1, "made for the shown door");
    app.world_mut().resource_mut::<DoorCamera>().manual = None;
    for _ in 0..4 {
        app.update();
    }
    assert_eq!(
        cameras(&mut app),
        1,
        "still there after a few quiet seconds"
    );
    for _ in 0..4 {
        app.update();
    }
    assert_eq!(cameras(&mut app), 0, "gone after five");
    assert!(app.world().resource::<DoorCamera>().camera.is_none());
    app.world_mut().resource_mut::<DoorCamera>().manual = Some(1);
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(cameras(&mut app), 1, "and made again");
}
