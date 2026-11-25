//! Planning pass for scheduled traffic vehicles.

use std::collections::HashMap;

use bevy::math::DVec2;
use bevy::prelude::*;
use opts_bevy::bus_stops::BusStopMarker;
use opts_traffic::{LaneId, LaneKind};

use super::super::run_schedule::{active_runs, run_phase, RunPhase, RunRef};
use super::super::{TrafficPool, TrafficSettings, TrafficState};
use super::path::{graph_leg_speed, leg, place_along, walk_back};
use super::{ScheduledRequest, ScheduledTraffic, MAX_SCHEDULED};
use crate::WorldLoadJob;

mod candidate;
mod pick;
mod route;
mod spot;
use pick::pick_slot_for;
use route::plan_route;
use spot::{appears_there, run_start, spawn_spot, PlanCtx, StopObject};

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

/// The clock, settings and scene queries the planner reads.
#[derive(bevy::ecs::system::SystemParam)]
pub struct PlanInputs<'w, 's> {
    time: Res<'w, Time<Real>>,
    env: Option<Res<'w, opts_bevy::SessionEnvironment>>,
    settings: Res<'w, TrafficSettings>,
    markers: Query<'w, 's, (&'static BusStopMarker, &'static GlobalTransform)>,
    camera: Query<'w, 's, &'static GlobalTransform, With<opts_bevy::MainCamera>>,
}

/// Plans the next scheduled vehicle.
pub fn plan_scheduled(
    job: Option<ResMut<WorldLoadJob>>,
    state: Res<TrafficState>,
    mut pool: ResMut<TrafficPool>,
    mut sched: ResMut<ScheduledTraffic>,
    inputs: PlanInputs,
) {
    let PlanInputs {
        time,
        env,
        settings,
        markers,
        camera,
    } = inputs;
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
    candidate::free_gone_runs(&mut sched, &state);
    candidate::refresh_active(&mut sched, &tt, (now, env.weekday()));
    if sched.driving() >= MAX_SCHEDULED || pool.depots.is_empty() {
        return;
    }
    let stops = candidate::stop_objects(&markers);
    if stops.is_empty() {
        return;
    }
    let graph = state.world.graph();
    let (centre, view) = candidate::camera_ground(cam);
    let ctx = PlanCtx {
        now,
        centre,
        view,
        graph,
        stops,
        tt: &tt,
    };
    let mut candidates = sched.active.clone();
    candidate::sort_nearest(&mut candidates, &ctx);
    let mut seen = [0_u32; 4];
    for cand in candidates {
        seen[0] += 1;
        match candidate::plan_candidate(&ctx, (&mut sched, &mut pool), cand) {
            Ok(request) => {
                sched.request = Some(request);
                return;
            }
            Err(stage) => seen[1..=stage].iter_mut().for_each(|n| *n += 1),
        }
    }
    tracing::debug!(
        candidates = seen[0],
        started = seen[1],
        placed = seen[2],
        in_range = seen[3],
        "scheduled buses: none placed this pass"
    );
}
