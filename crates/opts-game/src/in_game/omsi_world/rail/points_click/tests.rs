//! Unit tests for rail point clicks: throwing points, occupied points and unknown points.
use super::*;
use opts_sim::pedestrian::{FootLane, Gate, Travel};

const KEY: u64 = 9;

fn net() -> PedestrianNet {
    let lane = |a: Vec3, b: Vec3| FootLane::new(vec![a, b], Travel::Both, 1.0).unwrap();
    let gate = |position| Gate {
        switch: KEY,
        position,
    };
    let mut n = PedestrianNet::default();
    n.insert_tile(
        (0, 0),
        vec![
            lane(Vec3::new(0.0, 100.0, 0.0), Vec3::new(0.0, 200.0, 0.0)).with_gate(gate(0)),
            lane(Vec3::new(0.0, 100.0, 0.0), Vec3::new(50.0, 150.0, 0.0)).with_gate(gate(1)),
        ],
    );
    n
}

#[test]
fn a_click_throws_the_points_back_and_forth() {
    let mut n = net();
    let none = HashSet::new();
    assert_eq!(throw(&mut n, KEY, &none), Some(1));
    assert_eq!(n.switch_position(KEY), 1);
    assert_eq!(throw(&mut n, KEY, &none), Some(0));
    assert_eq!(n.switch_position(KEY), 0);
}

#[test]
fn occupied_points_stay_as_they_are() {
    let mut n = net();
    let busy = HashSet::from([KEY]);
    assert_eq!(throw(&mut n, KEY, &busy), None);
    assert_eq!(n.switch_position(KEY), 0);
}

#[test]
fn unknown_points_are_not_thrown() {
    let mut n = net();
    assert_eq!(throw(&mut n, 77, &HashSet::new()), None);
}

#[test]
fn a_click_on_the_tongue_message_reaches_the_points() {
    let mut world = World::new();
    world.insert_resource(RailNetwork(net()));
    world.init_resource::<Messages<SceneryClicked>>();
    world.init_resource::<Messages<RaiseAlert>>();
    let (x, y) = switch_place(KEY);
    let at = opts_bevy::coords::omsi_to_bevy_pos(Vec3::new(x, y, 0.0));
    let owner = world
        .spawn((
            GlobalTransform::from_translation(at),
            SceneryScripts::new(
                opts_sim::script::ScriptEngine::from_sources(&[("t.osc", "{init}\n{end}")]).0,
                &[],
                &[],
                Vec::new(),
            ),
        ))
        .id();
    let mut schedule = Schedule::default();
    schedule.add_systems(click_points_system);
    world.write_message(SceneryClicked {
        owner,
        trigger: "press".into(),
    });
    schedule.run(&mut world);
    assert_eq!(world.resource::<RailNetwork>().0.switch_position(KEY), 0);
    world.write_message(SceneryClicked {
        owner,
        trigger: TOGGLE_SWITCH.into(),
    });
    schedule.run(&mut world);
    assert_eq!(world.resource::<RailNetwork>().0.switch_position(KEY), 1);
}
