//! Rail points as shared state (M-0113): a points object (`[switch]`) has one position, trains
//! that approach ask for the way they want, and the position follows when no vehicle is on the
//! points. The position also drives the object's `Switch` variable, which the tongue meshes
//! animate.
//!
//! A train that finds the points set against it (another vehicle holds them) takes the way that
//! is open: it never leaves the rails, and the points never change under a vehicle.

use std::collections::{HashMap, HashSet};

use opts_bevy::coords::omsi_to_bevy_pos;
use opts_bevy::scenery_runtime::SceneryScripts;
use opts_sim::pedestrian::switches::switch_place;

use super::*;

/// Trains ask for their way from this far ahead (m).
const ASK_REACH_M: f32 = 150.0;
/// The `Switch` variable of a points object is matched to the object within this distance (m).
const OBJECT_MATCH_M: f32 = 1.0;

/// Sets the points that trains approach to the position they need, unless a vehicle is on them.
pub fn set_points_system(
    net: Option<ResMut<RailNetwork>>,
    players: Query<&RailPlayer>,
    ai: Query<&AiTrain>,
) {
    let Some(mut net) = net else {
        return;
    };
    if net.0.switches().next().is_none() {
        return;
    }
    let trains: Vec<&Train> = players
        .iter()
        .filter_map(|p| p.train())
        .chain(ai.iter().map(|t| &t.train))
        .collect();
    let occupied: HashSet<u64> = trains
        .iter()
        .flat_map(|t| t.lanes_occupied())
        .filter_map(|l| net.0.switch_of(l))
        .collect();
    let mut asks: Vec<(f32, u64, u8)> = trains
        .iter()
        .filter_map(|t| {
            let ahead = t.next_switch(&net.0, ASK_REACH_M)?;
            let gate = net.0.lane(ahead.wanted)?.gate?;
            Some((ahead.distance_m, gate.switch, gate.position))
        })
        .collect();
    asks.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut claimed = HashSet::new();
    for (_, switch, position) in asks {
        if occupied.contains(&switch) || !claimed.insert(switch) {
            continue;
        }
        if net.0.switch_position(switch) != position {
            net.0.set_switch_position(switch, position);
        }
    }
}

/// Writes the position of each points object into its `Switch` variable.
pub fn points_visual_system(
    net: Option<Res<RailNetwork>>,
    objects: Query<(Entity, &GlobalTransform, &SceneryScripts)>,
    mut written: Local<HashMap<u64, (Entity, u8)>>,
) {
    let Some(net) = net else {
        return;
    };
    for key in net.0.switches() {
        let position = net.0.switch_position(key);
        let known = written.get(&key).copied();
        if known.is_some_and(|(e, p)| p == position && objects.contains(e)) {
            continue;
        }
        let (x, y) = switch_place(key);
        let at = omsi_to_bevy_pos(Vec3::new(x, y, 0.0));
        let found = objects.iter().find(|(_, t, _)| {
            let p = t.translation();
            (p.x - at.x).hypot(p.z - at.z) <= OBJECT_MATCH_M
        });
        if let Some((entity, _, scripts)) = found {
            scripts.with_engine(|e| e.set("Switch", f32::from(position)));
            written.insert(key, (entity, position));
        }
    }
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn the_nearer_train_wins_when_two_want_different_ways() {
        let mut world = World::new();
        world.insert_resource(net());
        world.spawn(player(0, 10.0, Branch::Straight));
        world.spawn(player(0, 60.0, Branch::Right));
        run(&mut world);
        assert_eq!(world.resource::<RailNetwork>().0.switch_position(KEY), 1);
    }
}
