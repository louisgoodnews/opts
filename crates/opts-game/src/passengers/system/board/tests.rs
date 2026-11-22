//! The boarding path of a free drive (no trip, `bus = None`): a waiting person whose destination
//! the stop serves boards, and one whose destination the stop does not serve stays behind. Both
//! paths that take people on are driven: the unseen crowd (`board_unseen`) and the walkers at the
//! doors (`walk_to_doors`). The fixtures are in code; no installed game data is needed.

use super::fixtures::{open_entry, rig_at, zoo};
use super::*;
use crate::in_game::omsi_world::PedestrianPool;
use crate::passengers::crowd::StopCrowds;
use crate::passengers::destinations::draw_dest;
use crate::passengers::rig::{Unseen, Walker};
use crate::passengers::system::walk::walk_to_doors;
use bevy::ecs::system::SystemState;
use bevy::math::Affine3A;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;

#[test]
fn a_free_drive_boards_an_unseen_passenger_whose_destination_the_stop_serves() {
    let mut rig = rig_at(1);
    let served = draw_dest(&mut rig);
    rig.stop_id = Some(2);
    let unserved = draw_dest(&mut rig);
    assert_eq!(served, Some(zoo()), "stop 1 is served by the Zoo trip");
    assert_eq!(unserved, None, "stop 2 is served by no trip");
    // The person whose destination is not served comes first, so the search must skip them.
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

    // A free drive: no trip, so the bus's destination is None.
    board_unseen(&mut rig, &open_entry(), UNSEEN_BOARD_S, None);
    assert_eq!(rig.sim.passengers().len(), 1, "the served person boards");
    assert_eq!(rig.sim.passengers()[0].dest, Some(zoo()));
    assert_eq!(rig.unseen.len(), 1, "the unserved person stays behind");

    board_unseen(&mut rig, &open_entry(), UNSEEN_BOARD_S, None);
    assert_eq!(rig.sim.passengers().len(), 1, "no second boarding");
    assert_eq!(rig.unseen.len(), 1);
    assert_eq!(rig.unseen[0].dest, None);
}

#[test]
fn a_free_drive_boards_a_walking_passenger_whose_destination_the_stop_serves() {
    let mut world = World::new();
    world.init_resource::<PedestrianPool>();
    world.init_resource::<StopCrowds>();
    world.init_resource::<Assets<SkinnedMeshInverseBindposes>>();
    let vehicle = world.spawn(Transform::default()).id();
    let served_person = world.spawn(Transform::default()).id();
    let unserved_person = world.spawn(Transform::default()).id();

    let mut rig = rig_at(1);
    let served = draw_dest(&mut rig);
    rig.stop_id = Some(2);
    let unserved = draw_dest(&mut rig);
    assert_eq!(served, Some(zoo()));
    assert_eq!(unserved, None);

    // Both people stand at the door already (the walk is over), so they board or wait at once.
    let frames = [Affine3A::IDENTITY];
    let door = rig.door_world(0, &frames);
    let walker = |entity, dest: Option<Destination>, age| Walker {
        entity,
        leg_m: 0.0,
        phase: 0.0,
        position: door,
        target: door,
        entry: 0,
        age,
        stops: 2,
        walked_m: 0.0,
        dest,
    };
    rig.approaching.push(walker(unserved_person, unserved, 40));
    rig.approaching.push(walker(served_person, served, 30));

    let mut state: SystemState<PassengerEnv<'_, '_>> = SystemState::new(&mut world);
    {
        let mut env = state
            .get_mut(&mut world)
            .expect("the passenger environment");
        walk_to_doors(
            &mut env,
            &mut rig,
            (&frames, Affine3A::IDENTITY, vehicle, None),
            &open_entry(),
            0.05,
        );
    }
    state.apply(&mut world);

    assert_eq!(
        rig.sim.passengers().len(),
        1,
        "only the served person boards"
    );
    assert_eq!(rig.sim.passengers()[0].dest, Some(zoo()));
    assert_eq!(rig.actors.len(), 1, "the boarded person is drawn");
    assert_eq!(
        rig.approaching.len(),
        1,
        "the unserved person waits at the door"
    );
    assert_eq!(rig.approaching[0].entity, unserved_person);
    assert_eq!(rig.approaching[0].dest, None);
}
