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

use opts_bevy::bus_stops::BusStopMarker;
use opts_traffic::{AgentId, LaneId, StopPoint, TrafficEvent};

use super::run_schedule::{active_runs, run_phase, RunPhase, RunRef};
use super::*;

/// Most scheduled vehicles on the road at once.
pub const MAX_SCHEDULED: usize = 24;
/// Seconds before its start that a run gets its vehicle.
const LEAD_S: f64 = 90.0;
/// No run lasts longer than this (s).
const HORIZON_S: f64 = 3.0 * 3600.0;
/// Seconds between two planning passes (real time).
const PLAN_EVERY_S: f32 = 1.0;
/// Nearest (m) and farthest (m) from the camera a scheduled vehicle appears.
const APPEAR_MIN_M: f64 = 40.0;
const APPEAR_MAX_M: f64 = 280.0;
/// Within this distance (m) of the camera a vehicle does not appear in its plain view.
const VIEW_CLEAR_M: f64 = 150.0;
/// How far a stop object may stand from the lane (m) and how much its heading may differ (rad).
const LANE_REACH_M: f64 = 9.0;
const LANE_HEADING_TOL: f64 = 0.65;
/// Dwell limits (s) taken from `Busstops.cfg`.
const DWELL_MIN_S: f64 = 8.0;
const DWELL_MAX_S: f64 = 45.0;

/// A vehicle asked for, to be built by the spawner.
pub(super) struct ScheduledRequest {
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

/// A stop object of the map: where a bus stops (OMSI frame) and which way it faces.
struct StopObject {
    pos: DVec2,
    heading: f64,
}

/// Where the lane distance `d` along `route` (starting `s0` into its first lane) is: the lane
/// index in the route and the position on it.
pub(super) fn place_along(lengths: &[f64], s0: f64, d: f64) -> Option<(usize, f64)> {
    let mut left = d + s0;
    for (i, len) in lengths.iter().enumerate() {
        if left <= *len || i + 1 == lengths.len() {
            return Some((i, left.clamp(0.0, *len)));
        }
        left -= len;
    }
    None
}

/// The lanes from stop place `a` to stop place `b` (both included) and their lengths.
fn leg(
    graph: &opts_traffic::LaneGraph,
    a: (LaneId, f64),
    b: (LaneId, f64),
) -> Option<(Vec<LaneId>, Vec<f64>)> {
    let lanes = if a.0 == b.0 && b.1 >= a.1 {
        vec![a.0]
    } else {
        graph.shortest_route(a.0, b.0)?
    };
    let lengths = lanes
        .iter()
        .map(|l| graph.lane(*l).map_or(0.0, |l| l.length()))
        .collect();
    Some((lanes, lengths))
}

/// Plans the next scheduled vehicle.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn plan_scheduled(
    time: Res<Time<Real>>,
    env: Option<Res<opts_bevy::SessionEnvironment>>,
    job: Option<ResMut<WorldLoadJob>>,
    state: Res<TrafficState>,
    mut pool: ResMut<TrafficPool>,
    mut sched: ResMut<ScheduledTraffic>,
    settings: Res<TrafficSettings>,
    markers: Query<(&BusStopMarker, &GlobalTransform)>,
    camera: Query<&GlobalTransform, With<opts_bevy::MainCamera>>,
) {
    sched.timer += time.delta_secs();
    if sched.timer < PLAN_EVERY_S {
        return;
    }
    sched.timer = 0.0;
    let (Some(env), Some(mut job), Ok(cam)) = (env, job, camera.single()) else {
        return;
    };
    if settings.baseline == 0 || sched.request.is_some() {
        return;
    }
    let Some(tt) = job.timetables() else { return };
    let now = env.seconds_of_day();
    // Vehicles that are gone free their run.
    {
        let sched = &mut *sched;
        sched
            .agent_run
            .retain(|id, _| state.world.agent(*id).is_some());
        for e in sched.runs.values_mut() {
            if e.agent.is_some_and(|a| state.world.agent(a).is_none()) {
                e.agent = None;
            }
        }
    }
    sched.refresh += PLAN_EVERY_S;
    if sched.refresh >= 5.0 || sched.active.is_empty() {
        sched.refresh = 0.0;
        sched.active = active_runs(&tt, now, LEAD_S, HORIZON_S, Some(env.weekday()));
        let keep: std::collections::HashSet<RunRef> =
            sched.active.iter().map(|(r, _)| r.clone()).collect();
        sched
            .runs
            .retain(|r, e| e.agent.is_some() || keep.contains(r));
    }
    if sched.driving() >= MAX_SCHEDULED || pool.depots.is_empty() {
        return;
    }
    // The stop objects of the loaded tiles by station id.
    let stops: HashMap<u32, StopObject> = markers
        .iter()
        .filter_map(|(m, g)| {
            let id = u32::try_from(m.id_code).ok()?;
            let t = g.translation();
            let f = g.rotation() * Vec3::NEG_Z;
            Some((
                id,
                StopObject {
                    pos: DVec2::new(f64::from(t.x), -f64::from(t.z)),
                    heading: f64::from(f.x).atan2(-f64::from(f.z)),
                },
            ))
        })
        .collect();
    if stops.is_empty() {
        return;
    }
    let graph = state.world.graph();
    let centre = DVec2::new(
        f64::from(cam.translation().x),
        -f64::from(cam.translation().z),
    );
    let fwd = cam.forward();
    let view = DVec2::new(f64::from(fwd.x), -f64::from(fwd.z)).normalize_or_zero();
    let place = |station: u32| -> Option<(LaneId, f64)> {
        let o = stops.get(&station)?;
        graph
            .nearest_place(o.pos, o.heading, LANE_HEADING_TOL, LANE_REACH_M, |l| {
                l.kind == LaneKind::Road && l.length() > 4.0
            })
            .map(|(lane, s, _)| (lane, s))
    };
    let candidates = sched.active.clone();
    for (run, schedule) in candidates {
        if sched.rail_runs.contains(&run) {
            continue;
        }
        let entry = sched.runs.entry(run.clone()).or_default();
        if entry.agent.is_some() || entry.tries >= 3 {
            continue;
        }
        let phase = run_phase(&schedule, now);
        let (first_stop, offset) = match phase {
            RunPhase::NotStarted { wait_s } if wait_s <= LEAD_S => (0, None),
            RunPhase::AtStop { index } => (index, None),
            RunPhase::Between { from, fraction } => (from, Some(fraction)),
            _ => continue,
        };
        let Some(a) = schedule
            .stops
            .get(first_stop)
            .and_then(|s| place(s.station_id))
        else {
            continue;
        };
        // Where the vehicle is placed: at the stop, or on the way to the next one.
        let (spawn_lane, spawn_s, route_from) = match offset {
            None => (a.0, a.1, vec![]),
            Some(fraction) => {
                let Some(b) = schedule
                    .stops
                    .get(first_stop + 1)
                    .and_then(|s| place(s.station_id))
                else {
                    continue;
                };
                let Some((lanes, lengths)) = leg(graph, a, b) else {
                    continue;
                };
                let total: f64 = lengths.iter().sum::<f64>()
                    - a.1
                    - (lengths.last().copied().unwrap_or(0.0) - b.1);
                let Some((i, s)) = place_along(&lengths, a.1, fraction * total.max(0.0)) else {
                    continue;
                };
                (lanes[i], s, Vec::new())
            }
        };
        let Some(lane) = graph.lane(spawn_lane) else {
            continue;
        };
        let at = lane.geometry.point_at(spawn_s);
        let distance = at.distance(centre);
        let to = (at - centre).normalize_or_zero();
        if !(APPEAR_MIN_M..=APPEAR_MAX_M).contains(&distance)
            || (view.dot(to) > 0.5 && distance < VIEW_CLEAR_M)
        {
            continue;
        }
        // The route and the stops through everything that is on the loaded map.
        let mut route = route_from;
        let mut points = Vec::new();
        let mut arrivals = Vec::new();
        let mut prev: Option<(LaneId, f64)> = offset.is_some().then_some((spawn_lane, spawn_s));
        for (k, st) in schedule
            .stops
            .iter()
            .enumerate()
            .skip(first_stop + usize::from(offset.is_some()))
        {
            let Some(p) = place(st.station_id) else { break };
            if let Some(from) = prev {
                let Some((lanes, _)) = leg(graph, from, p) else {
                    break;
                };
                route.extend(lanes.into_iter().skip(1));
            }
            let dwell = tt
                .stop(st.station_id)
                .map_or(DWELL_MIN_S, |s| f64::from(s.dwell_s))
                .clamp(DWELL_MIN_S, DWELL_MAX_S);
            points.push(StopPoint::new(
                st.station_id,
                p.0,
                p.1,
                dwell,
                f64::from(st.departure_s),
            ));
            arrivals.push((st.station_id, f64::from(st.arrival_s)));
            prev = Some(p);
            let _ = k;
        }
        if points.is_empty() {
            continue;
        }
        // The vehicle: the tour's depot group.
        let Some(tour) = tt.tour(&schedule.line_file, run.tour) else {
            continue;
        };
        let Some(choices) = pool
            .depots
            .get(&tour.vehicle_group.to_ascii_lowercase())
            .cloned()
        else {
            sched.runs.entry(run).or_default().tries = 3;
            continue;
        };
        let ready: Vec<(usize, u32)> = choices
            .iter()
            .copied()
            .filter(|(slot, _)| matches!(pool.slots[*slot].state, SlotState::Ready(_)))
            .collect();
        if ready.is_empty() {
            // Ask for one to be parsed; the run is tried again next pass.
            if let Some((slot, _)) = choices
                .iter()
                .find(|(slot, _)| matches!(pool.slots[*slot].state, SlotState::Idle))
            {
                pool.slots[*slot].state = SlotState::Pending;
            } else if choices
                .iter()
                .all(|(slot, _)| matches!(pool.slots[*slot].state, SlotState::Failed))
            {
                sched.runs.entry(run).or_default().tries = 3;
            }
            continue;
        }
        let total: u32 = ready.iter().map(|(_, n)| n).sum();
        let mut pick = (run.run as u32 + run.tour as u32 * 7) % total.max(1);
        let slot = ready
            .iter()
            .find(|(_, n)| {
                if pick < *n {
                    true
                } else {
                    pick -= n;
                    false
                }
            })
            .map_or(ready[0].0, |(s, _)| *s);
        let speed = match (
            offset,
            schedule.stops.get(first_stop),
            schedule.stops.get(first_stop + 1),
        ) {
            (Some(_), Some(x), Some(y)) => {
                let secs = f64::from(y.arrival_s - x.departure_s).max(1.0);
                (graph_leg_speed(secs)).min(14.0)
            }
            _ => 0.0,
        };
        let entry = sched.runs.entry(run.clone()).or_default();
        entry.arrivals.clone_from(&arrivals);
        sched.request = Some(ScheduledRequest {
            run,
            slot,
            lane: spawn_lane,
            s: spawn_s,
            speed,
            route,
            stops: points,
            arrivals,
        });
        return;
    }
}

/// A plausible cruising speed (m/s) for a leg that takes `secs`: the vehicle will adapt.
fn graph_leg_speed(secs: f64) -> f64 {
    if secs < 30.0 {
        4.0
    } else {
        9.0
    }
}

#[cfg(test)]
mod tests;
