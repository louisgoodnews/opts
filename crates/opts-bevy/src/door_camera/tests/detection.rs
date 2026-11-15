//! Tests of the door detection: which animated mesh is an open door and which door it is.

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
        delay: None,
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
            // The mesh entity sits at its unit's origin; the pivot is in its space.
            GlobalTransform::from(at),
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
