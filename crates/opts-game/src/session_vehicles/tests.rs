//! Session vehicles without a window: switching, despawning, respawning, the place label, lost
//! vehicles and a session that never had a vehicle.

use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use opts_bevy::{
    ArticulatedChain, CameraMode, CameraState, PlayerVehicle, VehicleController,
};

use super::*;
use crate::session_vehicles::ops;
use crate::app_state::AppState;
use crate::in_game::omsi_world::WorldStats;
use crate::in_game::types::InGameEntity;

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::input::InputPlugin, StatesPlugin))
        .init_state::<AppState>()
        .init_resource::<WorldStats>()
        .init_resource::<CameraState>()
        .add_plugins(SessionVehiclesPlugin);
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::InGame);
    for _ in 0..3 {
        app.update();
    }
    app
}

/// A session vehicle that stands at `at` (session frame) and was spawned at entry `entry`.
fn add(app: &mut App, name: &str, entry: &str, at: Vec2) -> (VehicleId, Entity) {
    let mut controller = VehicleController::standard_bus(12_000.0, 5.9);
    controller.sim.chassis.position = at;
    let entity = app
        .world_mut()
        .spawn((controller, InGameEntity, Transform::default()))
        .id();
    let mut reg = app.world_mut().resource_mut::<SessionVehicles>();
    let id = reg.register_alive(
        SpawnParams {
            name: name.to_owned(),
            ..Default::default()
        },
        entity,
        Some((1, entry.to_owned())),
        Some(SpawnPoint {
            x: f64::from(at.x),
            y: f64::from(at.y),
            z: 0.0,
            heading_deg: 0.0,
        }),
        None,
    );
    app.world_mut().entity_mut(entity).insert(SessionVehicle(id));
    (id, entity)
}

fn players(app: &mut App) -> Vec<Entity> {
    let mut q = app.world_mut().query_filtered::<Entity, With<PlayerVehicle>>();
    q.iter(app.world()).collect()
}

fn key(app: &mut App, code: KeyCode, logical: bevy::input::keyboard::Key) {
    for state in [
        bevy::input::ButtonState::Pressed,
        bevy::input::ButtonState::Released,
    ] {
        app.world_mut()
            .write_message(bevy::input::keyboard::KeyboardInput {
                key_code: code,
                logical_key: logical.clone(),
                state,
                text: None,
                repeat: false,
                window: Entity::PLACEHOLDER,
            });
        run(app, 2);
    }
}

fn run(app: &mut App, frames: usize) {
    for _ in 0..frames {
        app.update();
    }
}

#[test]
fn switching_moves_the_player_marker_and_parks_the_previous_vehicle() {
    let mut app = app();
    let (a, ea) = add(&mut app, "A", "Depot", Vec2::ZERO);
    let (b, eb) = add(&mut app, "B", "Stop", Vec2::new(100.0, 0.0));
    ops::switch_to(app.world_mut(), a).expect("switch to a");
    assert_eq!(players(&mut app), vec![ea]);
    app.world_mut()
        .get_mut::<VehicleController>(ea)
        .expect("controller")
        .inputs_mut()
        .set_throttle(0.8);
    ops::switch_to(app.world_mut(), b).expect("switch to b");
    assert_eq!(players(&mut app), vec![eb], "exactly one active vehicle");
    let old = app.world().get::<VehicleController>(ea).expect("a");
    assert!(old.inputs().is_parking_brake_engaged(), "the previous vehicle is parked");
    assert_eq!(old.inputs().throttle(), 0.0);
    assert_eq!(app.world().resource::<SessionVehicles>().active_id(), Some(b));
    assert_eq!(app.world().resource::<WorldStats>().vehicle_name, "B");
    // The same vehicle again changes nothing; a gone one is refused.
    ops::switch_to(app.world_mut(), b).expect("no-op");
    assert!(matches!(ops::switch_to(app.world_mut(), 99), Err(OpError::NotFound(_))));
}

#[test]
fn quick_switch_cycles_through_the_living_vehicles() {
    let mut app = app();
    let (a, _) = add(&mut app, "A", "x", Vec2::ZERO);
    let (b, _) = add(&mut app, "B", "x", Vec2::ZERO);
    let (c, _) = add(&mut app, "C", "x", Vec2::ZERO);
    ops::switch_to(app.world_mut(), a).expect("a");
    assert_eq!(ops::switch_step(app.world_mut(), 1), Ok(b));
    assert_eq!(ops::switch_step(app.world_mut(), 1), Ok(c));
    assert_eq!(ops::switch_step(app.world_mut(), 1), Ok(a));
    assert_eq!(ops::switch_step(app.world_mut(), -1), Ok(c));
    // The key does it: F8 and Shift+F8.
    key(&mut app, QUICK_SWITCH_KEY, bevy::input::keyboard::Key::F8);
    assert_eq!(app.world().resource::<SessionVehicles>().active_id(), Some(a));
    key(&mut app, KeyCode::ShiftLeft, bevy::input::keyboard::Key::Shift);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ShiftLeft);
    key(&mut app, QUICK_SWITCH_KEY, bevy::input::keyboard::Key::F8);
    assert_eq!(app.world().resource::<SessionVehicles>().active_id(), Some(c));
}

#[test]
fn despawning_an_inactive_vehicle_leaves_the_active_one_alone() {
    let mut app = app();
    let (a, ea) = add(&mut app, "A", "x", Vec2::ZERO);
    let (b, eb) = add(&mut app, "B", "x", Vec2::ZERO);
    ops::switch_to(app.world_mut(), b).expect("b");
    let out = ops::despawn(app.world_mut(), a, DespawnReason::User).expect("despawn a");
    assert_eq!(out.active, Some(b));
    assert!(app.world().get_entity(ea).is_err(), "the entity is gone");
    assert_eq!(players(&mut app), vec![eb]);
    let rec = app.world().resource::<SessionVehicles>().get(a).cloned().expect("record");
    assert_eq!((rec.lifecycle, rec.despawn_reason), (Lifecycle::Despawned, Some(DespawnReason::User)));
    assert!(ops::despawn(app.world_mut(), a, DespawnReason::User).is_err(), "twice is refused");
}

#[test]
fn despawning_the_active_vehicle_activates_the_most_recent_other_or_none() {
    let mut app = app();
    let (a, _) = add(&mut app, "A", "x", Vec2::ZERO);
    let (b, _) = add(&mut app, "B", "x", Vec2::ZERO);
    let (c, ec) = add(&mut app, "C", "x", Vec2::ZERO);
    ops::switch_to(app.world_mut(), a).expect("a");
    let out = ops::despawn(app.world_mut(), a, DespawnReason::User).expect("despawn a");
    assert_eq!(out.active, Some(c), "the most recently spawned other one");
    assert_eq!(players(&mut app), vec![ec]);
    ops::despawn(app.world_mut(), c, DespawnReason::User).expect("despawn c");
    assert_eq!(app.world().resource::<SessionVehicles>().active_id(), Some(b));
    let out = ops::despawn(app.world_mut(), b, DespawnReason::User).expect("despawn b");
    assert_eq!(out.active, None);
    assert!(players(&mut app).is_empty());
    assert_eq!(app.world().resource::<CameraState>().mode, CameraMode::FreeRoam);
    assert_eq!(app.world().resource::<WorldStats>().vehicle_name, "");
}

#[test]
fn despawning_removes_the_trailer_chain_and_the_children() {
    let mut app = app();
    let (a, root) = add(&mut app, "A", "x", Vec2::ZERO);
    let seg = app.world_mut().spawn((InGameEntity, Transform::default())).id();
    let child = app.world_mut().spawn(InGameEntity).id();
    app.world_mut().entity_mut(root).add_child(child);
    app.world_mut()
        .entity_mut(root)
        .insert(ArticulatedChain { segments: vec![seg] });
    let out = ops::despawn(app.world_mut(), a, DespawnReason::User).expect("despawn");
    assert_eq!(out.removed, 2, "root and trailer");
    for e in [root, seg, child] {
        assert!(app.world().get_entity(e).is_err(), "{e:?} is gone");
    }
    let leftovers = app
        .world_mut()
        .query_filtered::<Entity, With<InGameEntity>>()
        .iter(app.world())
        .count();
    assert_eq!(leftovers, 0, "nothing of the vehicle is left");
}

#[test]
fn the_place_label_follows_the_spawn_place() {
    let mut app = app();
    assert_eq!(app.world().resource::<PlaceLabel>().0, None, "no vehicle, no label");
    let (a, ea) = add(&mut app, "A", "Rathaus", Vec2::new(10.0, 10.0));
    ops::switch_to(app.world_mut(), a).expect("a");
    run(&mut app, 2);
    assert_eq!(app.world().resource::<PlaceLabel>().0.as_deref(), Some("Rathaus"));
    // A few metres: still there.
    app.world_mut().get_mut::<VehicleController>(ea).expect("c").sim.chassis.position =
        Vec2::new(20.0, 10.0);
    run(&mut app, 2);
    assert_eq!(app.world().resource::<PlaceLabel>().0.as_deref(), Some("Rathaus"));
    // Further than the threshold: gone, and it stays gone when the vehicle comes back.
    app.world_mut().get_mut::<VehicleController>(ea).expect("c").sim.chassis.position =
        Vec2::new(10.0 + PLACE_LABEL_HIDE_DISTANCE_M + 5.0, 10.0);
    run(&mut app, 2);
    assert_eq!(app.world().resource::<PlaceLabel>().0, None);
    app.world_mut().get_mut::<VehicleController>(ea).expect("c").sim.chassis.position =
        Vec2::new(10.0, 10.0);
    run(&mut app, 2);
    assert_eq!(app.world().resource::<PlaceLabel>().0, None, "moved away once: hidden");
    // A respawn shows it again.
    ops::respawn(app.world_mut(), a).expect("respawn");
    run(&mut app, 2);
    assert_eq!(app.world().resource::<PlaceLabel>().0.as_deref(), Some("Rathaus"));
    // A new vehicle spawned at another place shows its own name once active; despawn hides it.
    let (b, _) = add(&mut app, "B", "Bahnhof", Vec2::new(500.0, 0.0));
    ops::switch_to(app.world_mut(), b).expect("b");
    run(&mut app, 2);
    assert_eq!(app.world().resource::<PlaceLabel>().0.as_deref(), Some("Bahnhof"));
    ops::despawn(app.world_mut(), b, DespawnReason::User).expect("despawn b");
    ops::despawn(app.world_mut(), a, DespawnReason::User).expect("despawn a");
    run(&mut app, 2);
    assert_eq!(app.world().resource::<PlaceLabel>().0, None);
}

#[test]
fn respawn_puts_the_vehicle_back_at_rest() {
    let mut app = app();
    let (a, ea) = add(&mut app, "A", "x", Vec2::new(5.0, 6.0));
    {
        let mut c = app.world_mut().get_mut::<VehicleController>(ea).expect("c");
        c.sim.chassis.position = Vec2::new(300.0, 300.0);
        c.sim.chassis.local_velocity = Vec2::new(0.0, 12.0);
    }
    ops::respawn(app.world_mut(), a).expect("respawn");
    let c = app.world().get::<VehicleController>(ea).expect("c");
    assert_eq!(c.sim.chassis.position, Vec2::new(5.0, 6.0));
    assert_eq!(c.sim.chassis.local_velocity, Vec2::ZERO);
}

#[test]
fn a_vehicle_that_vanishes_is_noticed_and_never_dangles() {
    let mut app = app();
    let (a, ea) = add(&mut app, "A", "x", Vec2::ZERO);
    let (b, eb) = add(&mut app, "B", "x", Vec2::ZERO);
    ops::switch_to(app.world_mut(), b).expect("b");
    app.world_mut().entity_mut(eb).despawn();
    run(&mut app, 2);
    let reg = app.world().resource::<SessionVehicles>();
    assert_eq!(reg.get(b).map(|r| r.despawn_reason), Some(Some(DespawnReason::Lost)));
    assert_eq!(reg.active_id(), Some(a), "the next vehicle took over");
    assert_eq!(players(&mut app), vec![ea]);
}

#[test]
fn the_session_ends_with_an_empty_registry() {
    let mut app = app();
    add(&mut app, "A", "x", Vec2::ZERO);
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::MainMenu);
    run(&mut app, 3);
    assert_eq!(app.world().resource::<SessionVehicles>().records().len(), 0);
}
