//! Planning pass for scheduled traffic vehicles.

use std::collections::HashMap;

use bevy::math::DVec2;
use bevy::prelude::*;
use opts_bevy::bus_stops::BusStopMarker;
use opts_traffic::{LaneId, LaneKind, StopPoint};

use super::super::run_schedule::{active_runs, run_phase, RunPhase, RunRef};
use super::super::{TrafficPool, TrafficSettings, TrafficState};
use super::path::{graph_leg_speed, leg, place_along, walk_back};
use super::{ScheduledRequest, ScheduledTraffic, MAX_SCHEDULED};
use crate::in_game::omsi_world::traffic_variant::SlotState;
use crate::in_game::omsi_world::WorldLoadJob;

/// Seconds before its start that a run gets its vehicle.
const LEAD_S: f64 = 90.0;
/// No run lasts longer than this (s).
const HORIZON_S: f64 = 3.0 * 3600.0;
/// Seconds between two planning passes (real time).
const PLAN_EVERY_S: f32 = 1.0;
/// Nearest (m) and farthest (m) from the camera a scheduled vehicle appears.
const APPEAR_MIN_M: f64 = 40.0;
const APPEAR_MAX_M: f64 = 450.0;
/// Within this distance (m) of the camera a vehicle does not appear in its plain view.
const VIEW_CLEAR_M: f64 = 150.0;
/// How far a stop object may stand from the lane (m) and how much its heading may differ (rad).
const LANE_REACH_M: f64 = 9.0;
const LANE_HEADING_TOL: f64 = 0.65;
/// A vehicle that comes from upstream of a loaded stop is put this far back (m/s times the seconds
/// until its arrival, at least `UPSTREAM_MIN_M`).
const UPSTREAM_MS: f64 = 10.0;
const UPSTREAM_MIN_M: f64 = 40.0;
/// Dwell limits (s) taken from `Busstops.cfg`.
const DWELL_MIN_S: f64 = 15.0;
const DWELL_MAX_S: f64 = 45.0;

/// A stop object of the map: where a bus stops (OMSI frame) and which way it faces.
struct StopObject {
    pos: DVec2,
    heading: f64,
}

/// What one planning pass knows about the world.
struct PlanCtx<'a> {
    now: f64,
    centre: DVec2,
    view: DVec2,
    graph: &'a opts_traffic::LaneGraph,
    stops: HashMap<u32, StopObject>,
    tt: &'a opts_core::formats::timetable::Timetables,
}

impl PlanCtx<'_> {
    /// The lane and distance along it where the stop object of `station` stands.
    fn place(&self, station: u32) -> Option<(LaneId, f64)> {
        let o = self.stops.get(&station)?;
        self.graph
            .nearest_place(o.pos, o.heading, LANE_HEADING_TOL, LANE_REACH_M, |l| {
                l.kind == LaneKind::Road && l.length() > 4.0
            })
            .map(|(lane, s, _)| (lane, s))
    }
}

/// Where a run is now: the stop it starts at and, between stops, how far it has come; `None` for
/// a run that is not about to start or already over.
fn run_start(ctx: &PlanCtx, schedule: &opts_core::formats::timetable::ActiveSchedule) -> Option<(usize, Option<f64>)> {
    let phase = run_phase(schedule, ctx.now);
    Some(match phase {
        RunPhase::NotStarted { wait_s } if wait_s <= LEAD_S => (0, None),
        RunPhase::AtStop { index } => (index, None),
        RunPhase::Between { from, fraction } => (from, Some(fraction)),
        _ => return None,
    })
}

/// Where the vehicle is placed: at the stop, or on the way to the next one; when the stop it
/// comes from is not on loaded lanes but the next one is, it comes from upstream of that.
fn spawn_spot(
    ctx: &PlanCtx,
    schedule: &opts_core::formats::timetable::ActiveSchedule,
    (first_stop, offset): (usize, Option<f64>),
) -> Option<(LaneId, f64, Vec<LaneId>)> {
    let (graph, now) = (ctx.graph, ctx.now);
    let a = schedule
        .stops
        .get(first_stop)
        .and_then(|s| ctx.place(s.station_id));
    Some(match (offset, a) {
        (None, Some(a)) => (a.0, a.1, vec![]),
        (None, None) => continue,
        (Some(fraction), a) => {
            let Some(b) = schedule
                .stops
                .get(first_stop + 1)
                .and_then(|s| ctx.place(s.station_id))
            else {
                return None;
            };
            match a {
                Some(a) => {
                    let Some((lanes, lengths)) = leg(graph, a, b) else {
                        return None;
                    };
                    let total: f64 = lengths.iter().sum::<f64>()
                        - a.1
                        - (lengths.last().copied().unwrap_or(0.0) - b.1);
                    let Some((i, s)) = place_along(&lengths, a.1, fraction * total.max(0.0))
                    else {
                        return None;
                    };
                    (lanes[i], s, Vec::new())
                }
                None => {
                    let due = schedule.stops[first_stop + 1].arrival_s;
                    let back = ((f64::from(due) - now).max(0.0) * UPSTREAM_MS)
                        .clamp(UPSTREAM_MIN_M, APPEAR_MAX_M);
                    let Some((lane, s)) = walk_back(graph, b.0, b.1, back) else {
                        return None;
                    };
                    (lane, s, Vec::new())
                }
            }
        }
    })
}

/// Whether a place is in the ring around the camera where vehicles appear, and not in plain view.
fn appears_there(ctx: &PlanCtx, (spawn_lane, spawn_s): (LaneId, f64)) -> bool {
    let Some(lane) = ctx.graph.lane(spawn_lane) else {
        return false;
    };
    let (centre, view) = (ctx.centre, ctx.view);
    let at = lane.geometry.point_at(spawn_s);
    let distance = at.distance(centre);
    let to = (at - centre).normalize_or_zero();
    if !(APPEAR_MIN_M..=APPEAR_MAX_M).contains(&distance)
        || (view.dot(to) > 0.5 && distance < VIEW_CLEAR_M)
    {
        continue;
    }
    true
}

/// The route and the stops through everything that is on the loaded map, from where the vehicle
/// appears: the lanes, the stop points and the planned arrivals.
fn plan_route(
    ctx: &PlanCtx,
    schedule: &opts_core::formats::timetable::ActiveSchedule,
    (first_stop, offset): (usize, Option<f64>),
    (spawn_lane, spawn_s, route_from): (LaneId, f64, Vec<LaneId>),
) -> (Vec<LaneId>, Vec<StopPoint>, Vec<(u32, f64)>) {
    let (graph, tt) = (ctx.graph, ctx.tt);
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
        let Some(p) = ctx.place(st.station_id) else { break };
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
    (route, points, arrivals)
}

/// The vehicle slot of the tour's depot group: one that is ready, chosen by the run. Asks for one
/// to be parsed when none is ready, and gives the run up when none can be.
fn pick_slot_for(
    (sched, pool): (&mut ScheduledTraffic, &mut TrafficPool),
    tt: &opts_core::formats::timetable::Timetables,
    (run, schedule): (&RunRef, &opts_core::formats::timetable::ActiveSchedule),
) -> Option<usize> {
    let Some(tour) = tt.tour(&schedule.line_file, run.tour) else {
        return None;
    };
    let Some(choices) = pool
        .depots
        .get(&tour.vehicle_group.to_ascii_lowercase())
        .cloned()
    else {
        sched.runs.entry(run.clone()).or_default().tries = 3;
        return None;
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
            sched.runs.entry(run.clone()).or_default().tries = 3;
        }
        return None;
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
    Some(slot)
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
    let ctx = PlanCtx {
        now,
        centre,
        view,
        graph,
        stops,
        tt: &tt,
    };
    let mut candidates = sched.active.clone();
    // The runs nearest to the player first: a bus the player can see beats one at the edge.
    let near = |run: &(RunRef, opts_core::formats::timetable::ActiveSchedule)| -> f64 {
        let idx = match run_phase(&run.1, now) {
            RunPhase::Between { from, .. } => from + 1,
            RunPhase::AtStop { index } => index,
            _ => 0,
        };
        run.1
            .stops
            .get(idx)
            .and_then(|s| ctx.stops.get(&s.station_id))
            .map_or(f64::MAX, |o| o.pos.distance(centre))
    };
    candidates.sort_by(|a, b| near(a).total_cmp(&near(b)));
    let mut seen = [0_u32; 4];
    for (run, schedule) in candidates {
        seen[0] += 1;
        if sched.rail_runs.contains(&run) || sched.player_run.as_ref() == Some(&run) {
            continue;
        }
        let entry = sched.runs.entry(run.clone()).or_default();
        if entry.agent.is_some() || entry.tries >= 3 {
            continue;
        }
        let Some((first_stop, offset)) = run_start(&ctx, &schedule) else {
            continue;
        };
        seen[1] += 1;
        let Some((spawn_lane, spawn_s, route_from)) = spawn_spot(&ctx, &schedule, (first_stop, offset))
        else {
            continue;
        };
        seen[2] += 1;
        if !appears_there(&ctx, (spawn_lane, spawn_s)) {
            continue;
        }
        seen[3] += 1;
        let (route, points, arrivals) = plan_route(
            &ctx,
            &schedule,
            (first_stop, offset),
            (spawn_lane, spawn_s, route_from),
        );
        if points.is_empty() {
            continue;
        }
        let Some(slot) = pick_slot_for((&mut sched, &mut pool), ctx.tt, (&run, &schedule)) else {
            continue;
        };
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
    tracing::debug!(
        candidates = seen[0],
        started = seen[1],
        placed = seen[2],
        in_range = seen[3],
        "scheduled buses: none placed this pass"
    );
}
