//! Route planning, stop-distance control and retirement for scheduled trains and trams.

use super::*;

/// Longest way (m) over the tracks between two stops.
const LEG_LIMIT_M: f32 = 25_000.0;
/// A train brakes at this rate (m/s²) to its stop point, within `LOOKAHEAD_M`.
const BRAKE_MS2: f32 = 1.0;
const LOOKAHEAD_M: f32 = 300.0;
/// Arrived: within this distance (m) of the stop point and slower than `ARRIVED_MS` (m/s).
const ARRIVE_WITHIN_M: f32 = 2.0;
const ARRIVED_MS: f32 = 0.3;
/// A train is removed this long (s) after arriving at its last stop, if the camera is farther
/// than `RETIRE_FAR_M` (m) away.
const RETIRE_AFTER_S: f64 = 8.0;
const RETIRE_FAR_M: f32 = 80.0;

/// How far a train is from a place on a lane.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in super::super) enum Ahead {
    /// This far (m) along the track.
    At(f32),
    /// Beyond the look-ahead.
    Far,
    /// Passed, or not reachable.
    Gone,
}

pub(in super::super) fn distance_to(
    train: &Train,
    net: &PedestrianNet,
    lane: LaneId,
    s: f32,
) -> Ahead {
    if train.lane == lane {
        let d = (s - train.s) * f32::from(train.dir);
        return if d >= -3.0 {
            Ahead::At(d.max(0.0))
        } else {
            Ahead::Gone
        };
    }
    let mut sim = train.clone();
    let mut travelled = 0.0;
    while travelled < LOOKAHEAD_M {
        if sim.advance(3.0, net) == Moved::EndOfTrack {
            return Ahead::Gone;
        }
        travelled += 3.0;
        if sim.lane == lane {
            let rest = (s - sim.s) * f32::from(sim.dir);
            return Ahead::At((travelled + rest).max(0.0));
        }
    }
    Ahead::Far
}

/// The speed limit (m/s) a scheduled train must keep to stop where its run says, `None` for no
/// limit. Records arrivals and departures.
pub(in super::super) fn control(
    t: &mut AiTrain,
    net: &PedestrianNet,
    clock: f64,
    sched: &mut ScheduledTraffic,
) -> Option<f32> {
    let speed = t.speed;
    let train = &t.train;
    let plan = t.plan.as_mut()?;
    let stop = plan.stops.front_mut()?;
    if let Some(arrived) = stop.arrived_at {
        if clock >= (arrived + f64::from(stop.dwell_s)).max(stop.depart_not_before) {
            sched.rail_departed();
            plan.stops.pop_front();
            return None;
        }
        return Some(0.0);
    }
    match distance_to(train, net, stop.lane, stop.s) {
        Ahead::Gone => {
            sched.rail_missed();
            plan.stops.pop_front();
            None
        }
        Ahead::Far => None,
        Ahead::At(d) => {
            if d <= ARRIVE_WITHIN_M && speed < ARRIVED_MS {
                stop.arrived_at = Some(clock);
                sched.rail_arrived(stop.station, stop.scheduled, clock);
                if plan.stops.len() == 1 {
                    plan.retire_at = Some(clock + RETIRE_AFTER_S);
                }
                return Some(0.0);
            }
            Some((2.0 * BRAKE_MS2 * (d - 0.5).max(0.0)).sqrt())
        }
    }
}

/// Whether a scheduled train that has reached its last stop may be removed now: its time has
/// come and the camera is far away.
pub(in super::super) fn retired(
    t: &AiTrain,
    net: &PedestrianNet,
    clock: f64,
    cam: Option<Vec3>,
) -> bool {
    let Some(at) = t.plan.as_ref().and_then(|p| p.retire_at) else {
        return false;
    };
    if clock < at {
        return false;
    }
    let near = cam.is_some_and(|c| {
        t.train
            .pose(net)
            .is_some_and(|(p, _)| c.distance(omsi_to_bevy_pos(p)) < RETIRE_FAR_M)
    });
    !near
}

/// The run of a scheduled train that is being removed, for the bookkeeping.
pub(in super::super) fn run_of(t: &AiTrain) -> Option<&RunRef> {
    t.plan.as_ref().map(|p| &p.run)
}

/// A stop object's place on the tracks: lane, arc length, position (session frame).
#[derive(Debug, Clone, Copy)]
pub(in super::super) struct Place {
    pub(in super::super) lane: LaneId,
    pub(in super::super) s: f32,
}

/// The walking coordinate along `states` (lanes with their direction) of arc length `s` on lane
/// number `i`: metres from the start of the first lane in walking direction.
pub(in super::super) fn walk_u(
    net: &PedestrianNet,
    states: &[(LaneId, i8)],
    i: usize,
    s: f32,
) -> f32 {
    let before: f32 = states[..i]
        .iter()
        .map(|(l, _)| net.lane(*l).map_or(0.0, |l| l.length()))
        .sum();
    let len = net.lane(states[i].0).map_or(0.0, |l| l.length());
    before + if states[i].1 > 0 { s } else { len - s }
}

/// The lane index and arc length at walking coordinate `u` along `states`.
pub(in super::super) fn walk_at(
    net: &PedestrianNet,
    states: &[(LaneId, i8)],
    u: f32,
) -> Option<(usize, f32)> {
    let mut left = u.max(0.0);
    for (i, (l, d)) in states.iter().enumerate() {
        let len = net.lane(*l)?.length();
        if left <= len || i + 1 == states.len() {
            let left = left.min(len);
            return Some((i, if *d > 0 { left } else { len - left }));
        }
        left -= len;
    }
    None
}

/// The tracks a run takes through its stops.
pub(in super::super) struct RailRoute {
    /// The lanes in running order, each with the direction it is walked in; the first is the
    /// first stop's.
    pub(in super::super) states: Vec<(LaneId, i8)>,
    /// Index in `states` of the lane of each stop (parallel to the places).
    pub(in super::super) stop_index: Vec<usize>,
    /// Walking coordinate of each stop object's place.
    pub(in super::super) stop_u: Vec<f32>,
}

/// Whether route `a` is better than `b`: more stops reached, then the shorter way.
pub(in super::super) fn better(a: &RailRoute, b: &RailRoute) -> bool {
    match a.stop_u.len().cmp(&b.stop_u.len()) {
        std::cmp::Ordering::Greater => true,
        std::cmp::Ordering::Less => false,
        std::cmp::Ordering::Equal => {
            a.stop_u.last().unwrap_or(&f32::MAX) < b.stop_u.last().unwrap_or(&f32::MAX)
        }
    }
}

/// The way over the track network from `start` (lane, arc length) walking in direction `dir`
/// through the `targets`; the route ends at the last target that can be reached. `None` when the
/// first target cannot be reached.
pub(in super::super) fn build_route(
    net: &PedestrianNet,
    (start, dir): (Place, i8),
    targets: &[Place],
) -> Option<RailRoute> {
    let mut states = vec![(start.lane, dir)];
    let mut stop_index = Vec::new();
    let mut stop_u = Vec::new();
    let mut at_s = start.s;
    for t in targets {
        let (from, d) = *states.last()?;
        if from == t.lane {
            // On the same lane: ahead of the head if the walk goes that way (a few metres of
            // slack for a stop object beside the head).
            if (t.s - at_s) * f32::from(d) < -3.0 {
                break;
            }
        } else {
            match net.shortest_path(from, d, t.lane, LEG_LIMIT_M) {
                Some(p) => states.extend(p),
                None => break,
            }
        }
        let i = states.len() - 1;
        stop_index.push(i);
        stop_u.push(walk_u(net, &states, i, t.s));
        at_s = t.s;
    }
    (!stop_index.is_empty()).then_some(RailRoute {
        states,
        stop_index,
        stop_u,
    })
}
