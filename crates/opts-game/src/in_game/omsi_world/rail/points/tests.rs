//! Tests of the points: trains asking for their way, and the position that follows them.

use super::*;
use opts_sim::pedestrian::{FootLane, Gate, Travel};

const KEY: u64 = 9;

/// 100 m north, then points `KEY`: straight on (position 0) or north-east (position 1).
fn net() -> RailNetwork {
    let lane = |a: Vec3, b: Vec3| FootLane::new(vec![a, b], Travel::Both, 1.0).unwrap();
    let gate = |position| Gate {
        switch: KEY,
        position,
    };
    let mut n = opts_sim::pedestrian::PedestrianNet::default();
    n.insert_tile(
        (0, 0),
        vec![
            lane(Vec3::ZERO, Vec3::new(0.0, 100.0, 0.0)),
            lane(Vec3::new(0.0, 100.0, 0.0), Vec3::new(0.0, 200.0, 0.0)).with_gate(gate(0)),
            lane(Vec3::new(0.0, 100.0, 0.0), Vec3::new(50.0, 150.0, 0.0)).with_gate(gate(1)),
        ],
    );
    RailNetwork(n)
}

fn player(lane: opts_sim::pedestrian::LaneId, s: f32, branch: Branch) -> RailPlayer {
    let mut p = RailPlayer::new(10_000.0, 10.0);
    let mut t = Train::new(lane, s, 1);
    t.branch = branch;
    p.train = Some(t);
    p
}

fn run(world: &mut World) {
    let mut schedule = Schedule::default();
    schedule.add_systems(set_points_system);
    schedule.run(world);
}

#[test]
fn an_approaching_train_sets_the_points_to_its_way() {
    let mut world = World::new();
    world.insert_resource(net());
    world.spawn(player(0, 40.0, Branch::Right));
    run(&mut world);
    assert_eq!(world.resource::<RailNetwork>().0.switch_position(KEY), 1);
}

#[test]
fn the_points_do_not_change_while_a_train_is_on_them() {
    let mut world = World::new();
    world.insert_resource(net());
    // One train already on the straight lane of the points, one coming that wants the other way.
    world.spawn(player(1, 20.0, Branch::Straight));
    world.spawn(player(0, 40.0, Branch::Right));
    run(&mut world);
    assert_eq!(world.resource::<RailNetwork>().0.switch_position(KEY), 0);
}

/// Drives the train 5 m at a time with the points system running before each step; returns
/// the lane it ends on.
fn drive(world: &mut World, train: Entity, metres: f32) -> opts_sim::pedestrian::LaneId {
    for _ in 0..(metres / 5.0) as usize {
        run(world);
        world.resource_scope(|world, net: Mut<RailNetwork>| {
            let mut player = world.get_mut::<RailPlayer>(train).expect("player");
            player.train.as_mut().expect("train").advance(5.0, &net.0);
        });
    }
    world
        .get::<RailPlayer>(train)
        .unwrap()
        .train()
        .unwrap()
        .lane
}

#[test]
fn a_train_reaches_the_branch_it_wants_whichever_way_the_points_stood() {
    for (branch, start, lane) in [
        (Branch::Right, 0, 2),
        (Branch::Straight, 1, 1),
        (Branch::Right, 1, 2),
        (Branch::Straight, 0, 1),
    ] {
        let mut world = World::new();
        let mut rail = net();
        rail.0.set_switch_position(KEY, start);
        world.insert_resource(rail);
        let train = world.spawn(player(0, 20.0, branch)).id();
        assert_eq!(
            drive(&mut world, train, 120.0),
            lane,
            "{branch:?} from position {start}"
        );
        let position = world.resource::<RailNetwork>().0.switch_position(KEY);
        assert_eq!(
            position,
            u8::from(lane == 2),
            "the points stand as the train needed"
        );
    }
}

#[test]
fn a_train_that_finds_the_points_held_against_it_takes_the_open_way() {
    let mut world = World::new();
    world.insert_resource(net());
    // Another train stands on the straight lane of the points and holds them at position 0.
    world.spawn(player(1, 20.0, Branch::Straight));
    let train = world.spawn(player(0, 20.0, Branch::Right)).id();
    assert_eq!(
        drive(&mut world, train, 120.0),
        1,
        "it stays on the rails, on the open way"
    );
    assert_eq!(
        world.resource::<RailNetwork>().0.switch_position(KEY),
        0,
        "never changed under a train"
    );
    // Once both trains have left the points, they follow the next train that asks.
    let trains: Vec<Entity> = world
        .query::<(Entity, &RailPlayer)>()
        .iter(&world)
        .map(|(e, _)| e)
        .collect();
    for e in trains {
        world.despawn(e);
    }
    let again = world.spawn(player(0, 20.0, Branch::Right)).id();
    assert_eq!(drive(&mut world, again, 120.0), 2);
    assert_eq!(world.resource::<RailNetwork>().0.switch_position(KEY), 1);
}

#[test]
fn the_nearer_train_wins_when_two_want_different_ways() {
    let mut world = World::new();
    world.insert_resource(net());
    world.spawn(player(0, 10.0, Branch::Straight));
    world.spawn(player(0, 60.0, Branch::Right));
    run(&mut world);
    assert_eq!(world.resource::<RailNetwork>().0.switch_position(KEY), 1);
}
