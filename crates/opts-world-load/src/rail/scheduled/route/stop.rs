//! Stop control of a scheduled train: the speed limit that brings it to its stop point, and the
//! arrival and departure bookkeeping.

use super::super::*;

use super::RETIRE_AFTER_S;

/// A train brakes at this rate (m/s²) to its stop point, within `LOOKAHEAD_M`.
const BRAKE_MS2: f32 = 1.0;
const LOOKAHEAD_M: f32 = 300.0;
/// Arrived: within this distance (m) of the stop point and slower than `ARRIVED_MS` (m/s).
const ARRIVE_WITHIN_M: f32 = 2.0;
const ARRIVED_MS: f32 = 0.3;

/// How far a train is from a place on a lane.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in super::super::super) enum Ahead {
    /// This far (m) along the track.
    At(f32),
    /// Beyond the look-ahead.
    Far,
    /// Passed, or not reachable.
    Gone,
}

pub(in super::super::super) fn distance_to(
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
pub(in super::super::super) fn control(
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
