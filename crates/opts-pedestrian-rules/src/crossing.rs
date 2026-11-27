//! Signal-controlled crossings: a walker waits where it would step onto a crossing whose light
//! is at stop.
//!
//! The stop line of a crossing is the junction point at each end of its footpath lane (a walker
//! may come from either side). A light's aspect is set per frame with [`CrossingGates::set_aspect`]:
//! only [`Aspect::Stop`] (red, or red with yellow) closes the crossing; green, yellow and dark
//! lights let walkers pass (the yellow choice is ❓ in the design, see `docs/src/simulation/pedestrians.md`).

use std::collections::{HashMap, HashSet};

use glam::Vec3;
use opts_signals::{Aspect, SignalKey};
use opts_sim_pedestrian::pedestrian::lane::FootLane;
use opts_sim_pedestrian::pedestrian::net::{End, LaneId, PedestrianNet, Walker};

/// A point quantised to millimetres, so that the same junction point always gives one key.
type PointKey = (i64, i64, i64);

fn point_key(p: Vec3) -> PointKey {
    let q = |v: f32| (f64::from(v) * 1000.0).round() as i64;
    (q(p.x), q(p.y), q(p.z))
}

/// The key of entering `lane` at one of its ends: the end point and the next vertex. The next
/// vertex tells the crossing apart from the sidewalk that meets it at the same junction point.
fn entry_key(lane: &FootLane, at_end: bool) -> (PointKey, PointKey) {
    let n = lane.points.len();
    let (p, next) = if at_end {
        (lane.points[n - 1], lane.points[n - 2])
    } else {
        (lane.points[0], lane.points[1])
    };
    (point_key(p), point_key(next))
}

/// The stop lines of the signal-controlled crossings and the lights that are at stop.
#[derive(Debug, Default)]
pub struct CrossingGates {
    stop_lines: HashMap<(PointKey, PointKey), SignalKey>,
    stopped: HashSet<SignalKey>,
}

impl CrossingGates {
    /// Registers the crossing lane `lane` as governed by the light `key`: both of its ends are
    /// stop lines of that light.
    pub fn add_crossing(&mut self, lane: &FootLane, key: SignalKey) {
        for at_end in [false, true] {
            self.stop_lines.insert(entry_key(lane, at_end), key);
        }
    }

    /// Removes the stop lines that `add_crossing` registered for `lane` and `key` (a tile was
    /// unloaded). Stop lines of other lights at the same points are kept.
    pub fn remove_crossing(&mut self, lane: &FootLane, key: SignalKey) {
        for at_end in [false, true] {
            let stop = entry_key(lane, at_end);
            if self.stop_lines.get(&stop) == Some(&key) {
                self.stop_lines.remove(&stop);
            }
        }
    }

    /// Sets the aspect of the light `key` for the frame: [`Aspect::Stop`] closes its crossings,
    /// any other aspect opens them.
    pub fn set_aspect(&mut self, key: SignalKey, aspect: Aspect) {
        if aspect == Aspect::Stop {
            self.stopped.insert(key);
        } else {
            self.stopped.remove(&key);
        }
    }

    /// Whether `walker` must hold this step: it would reach the end of its lane within this
    /// step (`dt` seconds) and one of the lanes it may continue onto there is a crossing at
    /// stop. A walker that is still further away keeps walking.
    #[must_use]
    pub fn holds(&self, net: &PedestrianNet, walker: &Walker, dt: f32) -> bool {
        let Some(lane) = net.lane(walker.lane) else {
            return false;
        };
        let (end, room) = if walker.dir > 0 {
            (End::End, lane.length() - walker.s)
        } else {
            (End::Start, walker.s)
        };
        if room > walker.speed * dt {
            return false;
        }
        net.continuations(walker.lane, end)
            .into_iter()
            .any(|(id, entry)| self.is_closed_at(net, id, entry))
    }

    /// Whether the lane `id` entered at `entry` is a crossing whose light is at stop.
    fn is_closed_at(&self, net: &PedestrianNet, id: LaneId, entry: End) -> bool {
        net.lane(id)
            .and_then(|l| self.stop_lines.get(&entry_key(l, entry == End::End)))
            .is_some_and(|key| self.stopped.contains(key))
    }
}
