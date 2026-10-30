//! The per-pass steps of the scheduled-traffic planner: bookkeeping, stop objects and the choice
//! of one candidate run.

use super::*;

/// A run with its schedule.
pub(super) type Candidate = (RunRef, opts_core::formats::timetable::ActiveSchedule);

/// Vehicles that are gone free their run.
pub(super) fn free_gone_runs(sched: &mut ScheduledTraffic, state: &TrafficState) {
    sched
        .agent_run
        .retain(|id, _| state.world.agent(*id).is_some());
    for e in sched.runs.values_mut() {
        if e.agent.is_some_and(|a| state.world.agent(a).is_none()) {
            e.agent = None;
        }
    }
}

/// Refreshes the list of active runs every few seconds.
pub(super) fn refresh_active(
    sched: &mut ScheduledTraffic,
    tt: &opts_core::formats::timetable::Timetables,
    (now, weekday): (f64, u32),
) {
    sched.refresh += PLAN_EVERY_S;
    if sched.refresh >= 5.0 || sched.active.is_empty() {
        sched.refresh = 0.0;
        sched.active = active_runs(tt, now, LEAD_S, HORIZON_S, Some(weekday));
        let keep: std::collections::HashSet<RunRef> =
            sched.active.iter().map(|(r, _)| r.clone()).collect();
        sched
            .runs
            .retain(|r, e| e.agent.is_some() || keep.contains(r));
    }
}

/// The stop objects of the loaded tiles by station id.
pub(super) fn stop_objects(
    markers: &Query<(&BusStopMarker, &GlobalTransform)>,
) -> HashMap<u32, StopObject> {
    markers
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
        .collect()
}

/// The camera position and viewing direction on the ground plane.
pub(super) fn camera_ground(cam: &GlobalTransform) -> (DVec2, DVec2) {
    let centre = DVec2::new(
        f64::from(cam.translation().x),
        -f64::from(cam.translation().z),
    );
    let fwd = cam.forward();
    let view = DVec2::new(f64::from(fwd.x), -f64::from(fwd.z)).normalize_or_zero();
    (centre, view)
}

/// The runs nearest to the player first: a bus the player can see beats one at the edge.
pub(super) fn sort_nearest(candidates: &mut [Candidate], ctx: &PlanCtx) {
    let near = |run: &Candidate| -> f64 {
        let idx = match run_phase(&run.1, ctx.now) {
            RunPhase::Between { from, .. } => from + 1,
            RunPhase::AtStop { index } => index,
            _ => 0,
        };
        run.1
            .stops
            .get(idx)
            .and_then(|s| ctx.stops.get(&s.station_id))
            .map_or(f64::MAX, |o| o.pos.distance(ctx.centre))
    };
    candidates.sort_by(|a, b| near(a).total_cmp(&near(b)));
}

/// The speed (m/s) a vehicle between two stops has at the start.
fn start_speed(
    schedule: &opts_core::formats::timetable::ActiveSchedule,
    (first_stop, offset): (usize, Option<f64>),
) -> f64 {
    match (
        offset,
        schedule.stops.get(first_stop),
        schedule.stops.get(first_stop + 1),
    ) {
        (Some(_), Some(x), Some(y)) => {
            let secs = f64::from(y.arrival_s - x.departure_s).max(1.0);
            (graph_leg_speed(secs)).min(14.0)
        }
        _ => 0.0,
    }
}

/// Plans one run: the request for its vehicle, or how many stages (0 to 3) of the pass it got
/// through before it was skipped.
pub(super) fn plan_candidate(
    ctx: &PlanCtx,
    (sched, pool): (&mut ScheduledTraffic, &mut TrafficPool),
    (run, schedule): Candidate,
) -> Result<ScheduledRequest, usize> {
    if sched.rail_runs.contains(&run) || sched.player_run.as_ref() == Some(&run) {
        return Err(0);
    }
    let entry = sched.runs.entry(run.clone()).or_default();
    if entry.agent.is_some() || entry.tries >= 3 {
        return Err(0);
    }
    let Some((first_stop, offset)) = run_start(ctx, &schedule) else {
        return Err(0);
    };
    let Some((spawn_lane, spawn_s, route_from)) = spawn_spot(ctx, &schedule, (first_stop, offset))
    else {
        return Err(1);
    };
    if !appears_there(ctx, (spawn_lane, spawn_s)) {
        return Err(2);
    }
    let (route, points, arrivals) = plan_route(
        ctx,
        &schedule,
        (first_stop, offset),
        (spawn_lane, spawn_s, route_from),
    );
    if points.is_empty() {
        return Err(3);
    }
    let Some(slot) = pick_slot_for((sched, pool), ctx.tt, (&run, &schedule)) else {
        return Err(3);
    };
    let speed = start_speed(&schedule, (first_stop, offset));
    let entry = sched.runs.entry(run.clone()).or_default();
    entry.arrivals.clone_from(&arrivals);
    Ok(ScheduledRequest {
        run,
        slot,
        lane: spawn_lane,
        s: spawn_s,
        speed,
        route,
        stops: points,
        arrivals,
    })
}
