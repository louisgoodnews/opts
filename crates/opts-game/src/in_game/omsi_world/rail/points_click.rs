//! The player throws rail points by clicking the tongue (M-0113): the `[mouseevent] toggle_switch`
//! of the points object. The click changes the shared position (the one the trains read and the
//! tongue animation follows through `points_visual_system`), never the object's `Switch`
//! variable alone, and never under a vehicle.
//!
//! A train that approaches asks for its own way afterwards (`set_points_system`), so a thrown
//! position holds until the next train that needs another one.

use std::collections::HashSet;

use opts_bevy::{SceneryClicked, SceneryScripts};
use opts_sim::pedestrian::switches::switch_place;
use opts_sim::pedestrian::PedestrianNet;

use super::points::{all_trains, occupied_points, OBJECT_MATCH_M};
use super::*;
use crate::hud::{AlertSeverity, RaiseAlert};

/// The script trigger of the tongue meshes.
pub const TOGGLE_SWITCH: &str = "toggle_switch";

/// The next position of `switch` (round robin over the positions its lanes use), or `None` when
/// it has a single position.
fn next_position(net: &PedestrianNet, switch: u64) -> Option<u8> {
    let count = net
        .switch_lanes(switch)
        .iter()
        .filter_map(|l| net.lane(*l).and_then(|l| l.gate))
        .map(|g| g.position + 1)
        .max()?;
    (count > 1).then(|| (net.switch_position(switch) + 1) % count)
}

/// Throws `switch` unless it is `occupied`; the new position, or `None` when it stays.
fn throw(net: &mut PedestrianNet, switch: u64, occupied: &HashSet<u64>) -> Option<u8> {
    if occupied.contains(&switch) {
        return None;
    }
    let next = next_position(net, switch)?;
    net.set_switch_position(switch, next);
    Some(next)
}

/// The points whose object stands at `at` (Bevy frame).
fn switch_at(net: &PedestrianNet, at: Vec3) -> Option<u64> {
    net.switches().find(|key| {
        let (x, y) = switch_place(*key);
        let p = opts_bevy::coords::omsi_to_bevy_pos(Vec3::new(x, y, 0.0));
        (p.x - at.x).hypot(p.z - at.z) <= OBJECT_MATCH_M
    })
}

/// Handles the clicks on a tongue.
pub fn click_points_system(
    mut clicks: MessageReader<SceneryClicked>,
    net: Option<ResMut<RailNetwork>>,
    objects: Query<&GlobalTransform, With<SceneryScripts>>,
    players: Query<&RailPlayer>,
    ai: Query<&AiTrain>,
    mut alerts: Option<MessageWriter<RaiseAlert>>,
) {
    let Some(mut net) = net else {
        clicks.clear();
        return;
    };
    for click in clicks.read().filter(|c| c.trigger == TOGGLE_SWITCH) {
        let Some(key) = objects
            .get(click.owner)
            .ok()
            .and_then(|t| switch_at(&net.0, t.translation()))
        else {
            continue;
        };
        let occupied = occupied_points(&net.0, &all_trains(&players, &ai));
        let (id, key_text, args) = match throw(&mut net.0, key, &occupied) {
            Some(p) => (
                "rail.points",
                crate::hud::keys::ALERT_POINTS_THROWN,
                vec![("position", (p + 1).to_string())],
            ),
            None if occupied.contains(&key) => (
                "rail.points",
                crate::hud::keys::ALERT_POINTS_OCCUPIED,
                Vec::new(),
            ),
            None => continue,
        };
        let Some(alerts) = alerts.as_mut() else {
            continue;
        };
        alerts.write(RaiseAlert {
            id,
            severity: AlertSeverity::Info,
            key: key_text,
            args,
        });
    }
}

#[cfg(test)]
mod tests {
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
}
