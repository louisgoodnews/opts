//! Tests for vehicle respawn, lost-entity recovery, session teardown and deferred despawn.

use super::*;

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
    run(&mut app, 6);
    let reg = app.world().resource::<SessionVehicles>();
    assert_eq!(
        reg.get(b).map(|r| r.despawn_reason),
        Some(Some(DespawnReason::Lost))
    );
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

/// What a system writes onto the player's vehicle every frame (like the passengers' feed).
#[derive(Component)]
struct EveryFrame;

fn write_every_frame(mut commands: Commands, players: Query<Entity, With<PlayerVehicle>>) {
    for e in &players {
        commands.entity(e).insert(EveryFrame);
    }
}

#[test]
fn a_confirmed_despawn_waits_for_the_frame_start_so_other_commands_never_hit_the_vehicle() {
    // The despawn confirmation used to despawn through a command of `despawn_choices`; the
    // command of a system after it (here: one that writes onto the vehicle each frame, as the
    // passengers do) was applied to a despawned entity and the game panicked.
    let mut app = app();
    // Both buffers are applied at the same sync point, the despawn's first: the order a frame
    // can have when the systems are not ordered.
    app.edit_schedule(Update, |s| {
        s.set_build_settings(bevy::ecs::schedule::ScheduleBuildSettings {
            auto_insert_apply_deferred: false,
            ..Default::default()
        });
    });
    app.add_systems(Update, write_every_frame.after(despawn_choices));
    let (a, ea) = add(&mut app, "A", "x", Vec2::ZERO);
    ops::switch_to(app.world_mut(), a).expect("a");
    run(&mut app, 2);
    app.world_mut().resource_mut::<PendingDespawn>().0 = Some(a);
    app.world_mut().write_message(DialogChoice {
        tag: DESPAWN_DIALOG,
        choice: DESPAWN,
    });
    run(&mut app, 3);
    assert!(app.world().get_entity(ea).is_err(), "the vehicle is gone");
    assert_eq!(
        app.world()
            .resource::<SessionVehicles>()
            .get(a)
            .map(|r| r.lifecycle),
        Some(Lifecycle::Despawned)
    );
    assert!(players(&mut app).is_empty());
}
