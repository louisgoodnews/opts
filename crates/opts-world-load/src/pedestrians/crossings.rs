//! Signal-controlled footpath crossings: the stop lines of the resident tiles, and the aspect of
//! each light, set once per frame from the traffic clock.
//!
//! The rules (stop lines, holds) live in `opts-pedestrian-rules`; this module only keeps them in
//! step with the tiles and the traffic lights.

use std::collections::HashMap;

use opts_pedestrian_rules::crossing::CrossingGates;
use opts_traffic::{Aspect, SignalKey};

use super::*;
use crate::traffic::TrafficState;

/// The crossings of the resident tiles: their stop lines, and the lanes registered per tile so
/// that an unloaded tile takes its crossings with it.
#[derive(Resource, Default)]
pub struct PedestrianCrossings {
    pub(super) gates: CrossingGates,
    tiles: HashMap<(i32, i32), Vec<(SignalKey, FootLane)>>,
}

impl PedestrianCrossings {
    /// Registers the crossings of a loaded tile.
    pub(super) fn insert_tile(&mut self, key: (i32, i32), crossings: Vec<(SignalKey, FootLane)>) {
        for (signal, lane) in &crossings {
            self.gates.add_crossing(lane, *signal);
        }
        self.tiles.insert(key, crossings);
    }

    /// Removes the crossings of an unloaded tile.
    pub(super) fn remove_tile(&mut self, key: (i32, i32)) {
        for (signal, lane) in self.tiles.remove(&key).unwrap_or_default() {
            self.gates.remove_crossing(&lane, signal);
        }
    }
}

/// Sets the aspect of every crossing's light from the traffic clock, before the walkers move.
/// A light whose program is not known is dark (`Off`): the crossing is open.
pub fn set_crossing_aspects(
    traffic: Option<Res<TrafficState>>,
    mut crossings: ResMut<PedestrianCrossings>,
) {
    let Some(traffic) = traffic else { return };
    let crossings = &mut *crossings;
    for (signal, _) in crossings.tiles.values().flatten() {
        let aspect = traffic.aspect(*signal).unwrap_or(Aspect::Off);
        crossings.gates.set_aspect(*signal, aspect);
    }
}
