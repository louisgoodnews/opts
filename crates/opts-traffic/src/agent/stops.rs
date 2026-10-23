//! Scheduled vehicles: a fixed route and stop points with a dwell and an earliest departure.
//!
//! A scheduled bus or tram does not wander: [`TrafficWorld::set_route`] gives it the lanes it
//! must take (the plan follows the route instead of choosing at random) and
//! [`TrafficWorld::push_stop`] the places it has to stop at, in running order. The first stop
//! acts as a stop line for the driver model, like a red light: the vehicle brakes to stand with
//! its front at the stop point, **arrives** (an event, with the clock time), stays at least the
//! dwell and until the scheduled departure, then **departs** (an event) and drives on to the
//! next stop. A stop whose lane the vehicle left without standing there is dropped with a
//! [`TrafficEvent::StopMissed`] so a route never gets stuck behind an unreachable stop.
//!
//! The clock is [`TrafficWorld::clock`] (seconds of the day), the one the timetable uses.

use super::*;

/// Within this distance (m) of its stop point, and slower than [`ARRIVED_SPEED`], a vehicle has
/// arrived.
pub const ARRIVE_WITHIN_M: f64 = 2.5;
/// A vehicle slower than this (m/s) counts as standing.
pub const ARRIVED_SPEED: f64 = 0.3;
/// The driver model aims this far (m) before the stop point.
const STOP_MARGIN_M: f64 = 0.4;

/// A place where a scheduled vehicle stops.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StopPoint {
    /// The station (`Busstops.cfg` id) the stop belongs to.
    pub station: u32,
    /// The lane the stop is on.
    pub lane: LaneId,
    /// Position of the front bumper along the lane when stopped (m).
    pub s: f64,
    /// Shortest time (s) the vehicle stays after arriving.
    pub min_dwell_s: f64,
    /// Earliest departure (clock, seconds of the day): the scheduled departure time.
    pub depart_not_before: f64,
    /// Clock time of the arrival, once there.
    pub arrived_at: Option<f64>,
}

impl StopPoint {
    /// A stop that is not reached yet.
    #[must_use]
    pub fn new(
        station: u32,
        lane: LaneId,
        s: f64,
        min_dwell_s: f64,
        depart_not_before: f64,
    ) -> Self {
        Self {
            station,
            lane,
            s,
            min_dwell_s,
            depart_not_before,
            arrived_at: None,
        }
    }

    /// Whether a vehicle on `lane` at `s` moving at `speed` has arrived here.
    #[must_use]
    pub fn reached(&self, lane: LaneId, s: f64, speed: f64) -> bool {
        lane == self.lane && self.s - s <= ARRIVE_WITHIN_M && speed < ARRIVED_SPEED
    }

    /// Whether the vehicle may leave at clock time `clock`.
    #[must_use]
    pub fn released(&self, clock: f64) -> bool {
        self.arrived_at
            .is_some_and(|t| clock >= (t + self.min_dwell_s).max(self.depart_not_before))
    }
}

impl TrafficWorld {
    /// Gives agent `id` the lanes it must take after its current one (and its plan). Lanes that
    /// do not follow one another are cut off where the chain breaks.
    pub fn set_route(&mut self, id: AgentId, lanes: impl IntoIterator<Item = LaneId>) -> bool {
        let Some(i) = self.agents.iter().position(|a| a.id == id) else {
            return false;
        };
        let a = &mut self.agents[i];
        a.route = lanes.into_iter().collect();
        // The plan chosen so far was a random one: plan again along the route.
        a.plan.clear();
        a.strategic = None;
        self.extend_plan(i);
        true
    }

    /// Appends a stop to the running order of agent `id`.
    pub fn push_stop(&mut self, id: AgentId, stop: StopPoint) -> bool {
        let Some(a) = self.agents.iter_mut().find(|a| a.id == id) else {
            return false;
        };
        a.stops.push_back(stop);
        true
    }

    /// The gap (m) from agent `i`'s front bumper to its next stop point when that lies within the
    /// lookahead: it limits the vehicle like a stop line.
    pub(super) fn stop_gap(&self, i: usize, look: &[(LaneId, f64)]) -> Option<f64> {
        let a = &self.agents[i];
        let stop = a.stops.front()?;
        // The driver model keeps its minimum gap before what it stops for: add it, so the front
        // bumper comes to rest at the stop point.
        look.iter()
            .find(|(lane, _)| *lane == stop.lane)
            .map(|(_, off)| (off + stop.s - STOP_MARGIN_M + a.params.min_gap).max(0.0))
    }

    /// Arrivals, departures and missed stops after a step (clock time of the world).
    pub(super) fn update_stops(&mut self) {
        let clock = self.clock();
        let mut events = Vec::new();
        for a in &mut self.agents {
            if a.foreign {
                continue;
            }
            while let Some(stop) = a.stops.front_mut() {
                if stop.arrived_at.is_none() && stop.reached(a.lane, a.s, a.speed) {
                    stop.arrived_at = Some(clock);
                    events.push(TrafficEvent::ArrivedAtStop {
                        id: a.id,
                        station: stop.station,
                        clock,
                    });
                }
                if stop.released(clock) {
                    events.push(TrafficEvent::DepartedStop {
                        id: a.id,
                        station: stop.station,
                        clock,
                    });
                    a.stops.pop_front();
                    continue;
                }
                // The lane of the stop is neither the vehicle's own, nor in its plan, nor on the
                // rest of its fixed route: it has been passed and cannot be reached any more.
                let ahead = a.lane == stop.lane
                    || a.plan.contains(&stop.lane)
                    || a.route.contains(&stop.lane);
                let behind = stop.arrived_at.is_none() && !ahead;
                if behind {
                    events.push(TrafficEvent::StopMissed {
                        id: a.id,
                        station: stop.station,
                    });
                    a.stops.pop_front();
                    continue;
                }
                break;
            }
        }
        self.events.extend(events);
    }

    /// The next lane of the fixed route when it continues from `last`; a route that does not
    /// follow the lanes is dropped.
    pub(super) fn next_route_lane(&mut self, i: usize, last: LaneId) -> Option<LaneId> {
        let next = *self.agents[i].route.front()?;
        if self.graph.successors(last).contains(&next) {
            self.agents[i].route.pop_front();
            Some(next)
        } else {
            self.agents[i].route.clear();
            None
        }
    }
}
