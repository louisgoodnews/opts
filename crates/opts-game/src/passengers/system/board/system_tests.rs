//! The whole `update_passengers` system, run on a free drive (a bus with no trip, `bus = None`)
//! at a stop: a waiting person whose destination the stop serves boards through the open entry,
//! and one whose destination it does not serve stays behind. The bus, its door readback and the
//! resources are built in code; no installed game data is needed.

use std::time::Duration;

use bevy::ecs::schedule::Schedule;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use opts_bevy::{PlayerVehicle, VehicleController, VehicleScripts};
use opts_sim::passengers::Unit;
use opts_sim::script::ScriptEngine;
use opts_sim::vehicle::VehicleSimulation;

use super::fixtures::{cabin, paths, rig_at, zoo};
use crate::in_game::omsi_world::PedestrianPool;
use crate::passengers::crowd::{StopCrowds, UNSEEN_BOARD_S};
use crate::passengers::destinations::draw_dest;
use crate::passengers::rig::{PassengerRig, Unseen};
use crate::passengers::update_passengers;

/// Runs `update_passengers` once on a free-drive bus whose entry 0 is open, with `rig` as its
/// passengers, and returns the rig as the system left it.
fn tick_free_drive(mut rig: PassengerRig) -> PassengerRig {
    let mut world = World::new();
    let mut time = Time::<()>::default();
    time.advance_by(Duration::from_millis(50));
    world.insert_resource(time);
    world.init_resource::<PedestrianPool>();
    world.init_resource::<StopCrowds>();
    world.init_resource::<Assets<SkinnedMeshInverseBindposes>>();

    // The bus is its own unit and the store of its door: its scripts report entry 0 open.
    let bus = world.spawn_empty().id();
    rig.add_unit(
        bus,
        Unit {
            cabin: cabin(),
            paths: paths(),
            ..Unit::default()
        },
        bus,
    );
    let (mut engine, _) = ScriptEngine::from_sources(&[]);
    engine.set("pax_entry0_open", 1.0);
    world.entity_mut(bus).insert((
        PlayerVehicle,
        rig,
        GlobalTransform::default(),
        VehicleController::new(VehicleSimulation::default()),
        VehicleScripts::new(engine),
    ));

    let mut schedule = Schedule::default();
    schedule.add_systems(update_passengers);
    schedule.run(&mut world);

    world
        .entity_mut(bus)
        .take::<PassengerRig>()
        .expect("the bus keeps its rig")
}

/// A rig at stop 1 (served by the Zoo trip) with one person bound for the Zoo and one with no
/// destination, the unserved one first, waiting at entry 0 and due to board.
fn rig_with_waiting_people() -> PassengerRig {
    let mut rig = rig_at(1);
    let served = draw_dest(&mut rig);
    rig.stop_id = Some(2);
    let unserved = draw_dest(&mut rig);
    assert_eq!(served, Some(zoo()), "stop 1 is served by the Zoo trip");
    assert_eq!(unserved, None, "stop 2 is served by no trip");
    rig.unseen.push(Unseen {
        entry: 0,
        age: 30,
        stops: 3,
        dest: unserved,
    });
    rig.unseen.push(Unseen {
        entry: 0,
        age: 40,
        stops: 2,
        dest: served,
    });
    rig.unseen_timer = UNSEEN_BOARD_S;
    rig
}

#[test]
fn update_passengers_boards_a_free_drive_passenger_whose_destination_the_stop_serves() {
    let rig = tick_free_drive(rig_with_waiting_people());

    assert_eq!(
        rig.sim.passengers().len(),
        1,
        "the served person boards through the open entry"
    );
    assert_eq!(rig.sim.passengers()[0].dest, Some(zoo()));
    assert_eq!(rig.unseen.len(), 1, "only one person is left waiting");
    assert_eq!(
        rig.unseen[0].dest, None,
        "the person whose destination the stop does not serve stays behind"
    );
}
