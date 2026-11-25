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
                opts_hud_keys::ALERT_POINTS_THROWN,
                vec![("position", (p + 1).to_string())],
            ),
            None if occupied.contains(&key) => (
                "rail.points",
                opts_hud_keys::ALERT_POINTS_OCCUPIED,
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
mod tests;
