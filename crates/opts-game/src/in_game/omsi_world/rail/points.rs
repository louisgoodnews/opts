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
pub(super) const OBJECT_MATCH_M: f32 = 1.0;

/// Every rail vehicle's train: the player's and the AI's.
pub(super) fn all_trains<'a>(
    players: &'a Query<&RailPlayer>,
    ai: &'a Query<&AiTrain>,
) -> Vec<&'a Train> {
    players
        .iter()
        .filter_map(|p| p.train())
        .chain(ai.iter().map(|t| &t.train))
        .collect()
}

/// The points that have a vehicle on them.
pub(super) fn occupied_points(
    net: &opts_sim::pedestrian::PedestrianNet,
    trains: &[&Train],
) -> HashSet<u64> {
    trains
        .iter()
        .flat_map(|t| t.lanes_occupied())
        .filter_map(|l| net.switch_of(l))
        .collect()
}

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
    let trains = all_trains(&players, &ai);
    let occupied = occupied_points(&net.0, &trains);
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
mod tests;
