//! Scheduled AI vehicles: runs of the map's timetable driven by depot-group vehicles.
//!
//! The timetable (`TTData`) says which run starts when, with which stops and times, and the tour
//! names the depot group (`ailists.cfg` `[aigroup_depot]`) whose vehicles drive it
//! (`docs/src/architecture/scheduled-traffic.md`). [`plan_scheduled`] looks at the runs that are on
//! the road at the session clock and, for one run at a time that has no vehicle yet and whose
//! stops lie near the player, asks [`spawn_traffic`](super::spawn_traffic) for a vehicle placed
//! where the run is by its times: at its first stop shortly before it starts, at a stop it is
//! standing at, or on the way between two stops. The vehicle gets the fixed route through the
//! stops that are in the loaded tiles and a stop point at each, with the dwell of `Busstops.cfg`
//! and the scheduled departure; the engine ([`opts_traffic::StopPoint`]) does the rest and
//! [`ScheduledTraffic::on_event`] keeps the punctuality statistics.
//!
//! The stops are the bus stop objects of the map (`BusStopMarker`, its ID code is the station
//! id of the timetable) in the resident tiles, put on the lane they stand beside.

use opts_traffic::{AgentId, LaneId, StopPoint, TrafficEvent};

mod path;
mod plan;
pub use plan::plan_scheduled;

use super::run_schedule::RunRef;
use super::*;

/// Most scheduled vehicles on the road at once.
pub const MAX_SCHEDULED: usize = 24;

/// A vehicle asked for, to be built by the spawner.
pub(crate) struct ScheduledRequest {
    pub run: RunRef,
    pub slot: usize,
    pub lane: LaneId,
    pub s: f64,
    pub speed: f64,
    pub route: Vec<LaneId>,
    pub stops: Vec<StopPoint>,
    /// Scheduled arrival (clock s) per station, for the punctuality figures.
    pub arrivals: Vec<(u32, f64)>,
}

/// A run with or without a vehicle.
#[derive(Default)]
struct RunEntry {
    agent: Option<AgentId>,
    /// Scheduled arrival per station of the run.
    arrivals: Vec<(u32, f64)>,
    /// A vehicle could not be built or placed this many times.
    tries: u32,
}

/// What the scheduled vehicles have done (`traffic.scheduled`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ScheduledStats {
    /// Vehicles built for a run.
    pub spawned: u32,
    /// Stops reached.
    pub arrivals: u32,
    /// Stops left.
    pub departures: u32,
    /// Stops dropped because the vehicle could not reach them.
    pub missed: u32,
    /// Sum of the delays at arrival (s; early is negative).
    pub delay_sum_s: f64,
    /// The last arrivals: `(station, scheduled clock, actual clock)`.
    pub log: Vec<(u32, f64, f64)>,
}

/// The scheduled traffic of the session.
#[derive(Resource, Default)]
pub struct ScheduledTraffic {
    timer: f32,
    refresh: f32,
    active: Vec<(RunRef, opts_core::formats::timetable::ActiveSchedule)>,
    runs: HashMap<RunRef, RunEntry>,
    agent_run: HashMap<AgentId, RunRef>,
    pub(super) request: Option<ScheduledRequest>,
    /// Vehicles that reached the last stop of their run: the clock time they may be removed at.
    retire: Vec<(AgentId, f64)>,
    /// Runs driven by a train or tram on the rails (`rail::scheduled`).
    rail_runs: std::collections::HashSet<RunRef>,
    /// The run the player's vehicle follows: no scheduled vehicle drives it.
    pub player_run: Option<RunRef>,
    /// The figures.
    pub stats: ScheduledStats,
}

impl ScheduledTraffic {
    /// Runs on the road or about to start, as the last pass saw them.
    #[must_use]
    pub fn active_runs(&self) -> usize {
        self.active.len()
    }

    /// The runs on the road or about to start with their schedules, as the last pass saw them.
    #[must_use]
    pub fn active_list(&self) -> &[(RunRef, opts_core::formats::timetable::ActiveSchedule)] {
        &self.active
    }

    /// Whether a train or tram drives `run` now.
    #[must_use]
    pub fn rail_has(&self, run: &RunRef) -> bool {
        self.rail_runs.contains(run)
    }

    /// Whether a road vehicle drives `run` now.
    #[must_use]
    pub fn road_has(&self, run: &RunRef) -> bool {
        self.runs.get(run).is_some_and(|r| r.agent.is_some())
    }

    /// Trains and trams on the road.
    #[must_use]
    pub fn rail_agents(&self) -> usize {
        self.rail_runs.len()
    }

    /// A train or tram was built for `run`.
    pub fn rail_started(&mut self, run: RunRef) {
        self.stats.spawned += 1;
        self.rail_runs.insert(run);
    }

    /// The train or tram of `run` is gone.
    pub fn rail_done(&mut self, run: &RunRef) {
        self.rail_runs.remove(run);
    }

    /// A train or tram stands at a stop of its run at `clock`; `scheduled` is the timetable's
    /// arrival there.
    pub fn rail_arrived(&mut self, station: u32, scheduled: f64, clock: f64) {
        self.stats.arrivals += 1;
        if (clock - scheduled).abs() >= 3600.0 {
            return;
        }
        self.stats.delay_sum_s += clock - scheduled;
        self.stats.log.push((station, scheduled, clock));
        if self.stats.log.len() > 40 {
            self.stats.log.remove(0);
        }
    }

    /// A train or tram left a stop.
    pub fn rail_departed(&mut self) {
        self.stats.departures += 1;
    }

    /// A train or tram dropped a stop it could not reach.
    pub fn rail_missed(&mut self) {
        self.stats.missed += 1;
    }

    /// Vehicles of scheduled runs on the road.
    #[must_use]
    pub fn agent_count(&self) -> usize {
        self.agent_run.len() + self.rail_runs.len()
    }

    /// Runs that have a vehicle now.
    #[must_use]
    pub fn driving(&self) -> usize {
        self.runs.values().filter(|r| r.agent.is_some()).count() + self.rail_runs.len()
    }

    /// The run of a vehicle, with its line text.
    #[must_use]
    pub fn run_of(&self, id: AgentId) -> Option<&RunRef> {
        self.agent_run.get(&id)
    }

    /// The vehicle was built: remember it for its run.
    pub(super) fn built(&mut self, id: AgentId, req: &ScheduledRequest) {
        self.stats.spawned += 1;
        self.agent_run.insert(id, req.run.clone());
        let e = self.runs.entry(req.run.clone()).or_default();
        e.agent = Some(id);
        e.arrivals.clone_from(&req.arrivals);
    }

    /// The vehicle could not be built.
    pub(super) fn failed(&mut self, req: &ScheduledRequest) {
        self.runs.entry(req.run.clone()).or_default().tries += 1;
    }

    /// Counts what the traffic engine reports about a scheduled vehicle.
    pub fn on_event(&mut self, e: &TrafficEvent) {
        match *e {
            TrafficEvent::ArrivedAtStop { id, station, clock } => {
                self.stats.arrivals += 1;
                let scheduled = self
                    .agent_run
                    .get(&id)
                    .and_then(|r| self.runs.get(r))
                    .and_then(|r| r.arrivals.iter().find(|(s, _)| *s == station))
                    .map(|(_, t)| *t);
                // A jump of the session clock (a run of before the jump) is not a delay.
                if let Some(t) = scheduled.filter(|t| (clock - t).abs() < 3600.0) {
                    self.stats.delay_sum_s += clock - t;
                    self.stats.log.push((station, t, clock));
                    if self.stats.log.len() > 40 {
                        self.stats.log.remove(0);
                    }
                }
            }
            TrafficEvent::DepartedStop { .. } => self.stats.departures += 1,
            TrafficEvent::StopMissed { .. } => self.stats.missed += 1,
            _ => {}
        }
    }

    /// Notes that scheduled vehicle `id` stands at the last stop of its run; it may be removed
    /// `after_s` seconds after `clock`.
    pub(super) fn finish_at(&mut self, id: AgentId, clock: f64, after_s: f64) {
        if self.agent_run.contains_key(&id) {
            self.retire.push((id, clock + after_s));
        }
    }

    /// The vehicles whose time to be removed has come (they are forgotten here).
    pub(super) fn take_retired(&mut self, clock: f64) -> Vec<AgentId> {
        let (due, rest): (Vec<_>, Vec<_>) = self.retire.drain(..).partition(|(_, t)| *t <= clock);
        self.retire = rest;
        due.into_iter().map(|(id, _)| id).collect()
    }

    /// Mean delay at arrival (s), `None` before the first arrival.
    #[must_use]
    pub fn mean_delay_s(&self) -> Option<f64> {
        (!self.stats.log.is_empty()).then(|| {
            self.stats.log.iter().map(|(_, s, a)| a - s).sum::<f64>() / self.stats.log.len() as f64
        })
    }
}

#[cfg(test)]
mod tests;
