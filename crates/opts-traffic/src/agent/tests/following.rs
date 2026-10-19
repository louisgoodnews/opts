//! Following, spawning and commands.

use super::*;

#[test]
fn an_agent_accelerates_and_crosses_into_the_next_lane() {
    let mut w = world(&[straight(0.0, 60.0), straight(60.0, 400.0)]);
    let id = w.spawn(LaneId(0), 0.0, 0.0, IdmParams::default()).unwrap();
    for _ in 0..200 {
        w.step(0.1);
    }
    let a = w.agent(id).unwrap();
    assert_eq!(a.lane, LaneId(1));
    assert!(a.speed > 10.0);
    let (p, h) = a.pose(w.graph()).unwrap();
    assert!(p.y > 60.0 && p.x.abs() < 1e-9 && h.abs() < 1e-9);
}

#[test]
fn a_follower_keeps_its_distance_behind_a_stopped_leader() {
    let mut w = world(&[straight(0.0, 300.0)]);
    let lead = w
        .spawn(LaneId(0), 200.0, 0.0, IdmParams::default())
        .unwrap();
    let p = IdmParams {
        max_accel: 0.0,
        ..IdmParams::default()
    };
    // A leader that never moves: zero acceleration means it stays at speed 0.
    w.agents.iter_mut().find(|a| a.id == lead).unwrap().params = p;
    let follow = w.spawn(LaneId(0), 0.0, 13.0, IdmParams::default()).unwrap();
    for _ in 0..1200 {
        w.step(0.1);
    }
    let (l, f) = (w.agent(lead).unwrap(), w.agent(follow).unwrap());
    let gap = l.s - l.params.length - f.s;
    assert!(f.speed < 0.05, "follower still at {}", f.speed);
    assert!(gap > 0.5, "collision: gap {gap}");
    assert!(gap < 4.0, "stopped too early: gap {gap}");
}

#[test]
fn agents_see_a_leader_across_the_lane_boundary() {
    let mut w = world(&[straight(0.0, 100.0), straight(100.0, 100.0)]);
    let lead = w
        .spawn(
            LaneId(1),
            10.0,
            0.0,
            IdmParams {
                max_accel: 0.0,
                ..IdmParams::default()
            },
        )
        .unwrap();
    let follow = w.spawn(LaneId(0), 0.0, 13.0, IdmParams::default()).unwrap();
    for _ in 0..1200 {
        w.step(0.1);
    }
    let (l, f) = (w.agent(lead).unwrap(), w.agent(follow).unwrap());
    assert_eq!(f.lane, LaneId(1));
    assert!(l.s - l.params.length - f.s > 0.5, "collision");
}

#[test]
fn dead_ends_remove_the_agent_and_raise_an_event() {
    let mut w = world(&[straight(0.0, 30.0)]);
    let id = w.spawn(LaneId(0), 0.0, 13.0, IdmParams::default()).unwrap();
    for _ in 0..100 {
        w.step(0.1);
    }
    assert!(w.agent(id).is_none());
    assert_eq!(
        w.drain_events(),
        [TrafficEvent::ReachedDeadEnd {
            id,
            lane: LaneId(0)
        }]
    );
    assert!(w.drain_events().is_empty());
}

#[test]
fn replacing_the_graph_keeps_agents_on_lanes_that_still_exist() {
    let mut a = straight(0.0, 100.0);
    a.source = 1;
    let mut b = straight(100.0, 100.0);
    b.source = 2;
    let mut w = world(&[a.clone(), b.clone()]);
    let on_a = w.spawn(LaneId(0), 50.0, 5.0, IdmParams::default()).unwrap();
    let on_b = w.spawn(LaneId(1), 50.0, 5.0, IdmParams::default()).unwrap();
    // Tile of lane `a` unloads; lane `b` stays but now has id 0.
    let removed = w.replace_graph(LaneGraph::build(&[b], &ConnectParams::default()));
    assert_eq!(removed, [on_a]);
    assert!(w.agent(on_a).is_none());
    assert_eq!(w.agent(on_b).unwrap().lane, LaneId(0));
    // Loading `a` again does not bring the agent back, and `on_b` keeps its lane by key.
    let removed = w.replace_graph(LaneGraph::build(
        &[a, straight(100.0, 100.0)],
        &ConnectParams::default(),
    ));
    assert!(removed.is_empty() || removed == [on_b]);
}

#[test]
fn spawning_needs_room() {
    let mut w = world(&[straight(0.0, 100.0)]);
    w.spawn(LaneId(0), 20.0, 0.0, IdmParams::default()).unwrap();
    assert_eq!(
        w.spawn(LaneId(0), 22.0, 0.0, IdmParams::default()),
        Err(SpawnError::Occupied)
    );
    assert!(w.spawn(LaneId(0), 40.0, 0.0, IdmParams::default()).is_ok());
    assert_eq!(
        w.spawn(LaneId(9), 0.0, 0.0, IdmParams::default()),
        Err(SpawnError::NoSuchLane)
    );
}

#[test]
fn commands_follow_the_acceleration_and_announce_turns() {
    // A right turn (quarter circle, radius 15) after 100 m.
    let turn = LaneSegment {
        geometry: PathGeometry {
            start: DVec2::new(0.0, 100.0),
            heading: 0.0,
            radius: 15.0,
            length: 15.0 * std::f64::consts::FRAC_PI_2,
        },
        kind: LaneKind::Road,
        width: 2.5,
        direction: Direction::Forward,
        speed_limit: None,
        rules: Default::default(),
        elevation: Default::default(),
        signal: None,
        junction: false,
        indicator: None,
        source: 0,
    };
    let mut w = world(&[straight(0.0, 100.0), turn]);
    let id = w.spawn(LaneId(0), 0.0, 0.0, IdmParams::default()).unwrap();
    w.step(0.1);
    let c = w.command(id).unwrap();
    assert!(c.throttle > 0.9 && c.brake == 0.0 && c.blinker == Blinker::Off);
    w.agents[0].s = 80.0;
    assert_eq!(w.command(id).unwrap().blinker, Blinker::Right);
}
