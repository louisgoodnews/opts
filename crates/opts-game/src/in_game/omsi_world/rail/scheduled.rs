//! Scheduled trains and trams: runs of the map's timetable driven on the rails (M-0112).
//!
//! The same runs as the scheduled road buses ([`super::super::traffic`]): the timetable says
//! when a run starts and where it stops, the tour names the group whose vehicles drive it. A
//! group of rail vehicles (`.zug` consists, trams of a depot group) goes on the track network
//! instead of the lane graph: [`spawn_scheduled_trains`] puts the train where its run is by the
//! clock, gives it the route over the tracks through the stops that are on loaded tiles
//! ([`opts_sim::pedestrian::PedestrianNet::shortest_path`]) and [`control`] holds it at each stop
//! for the dwell and until the scheduled departure. The stops are the bus stop objects of the map
//! (`BusStopMarker`), which platforms are too.

use std::collections::{HashMap, HashSet, VecDeque};

use super::ai::{AiTrain, SpawnAssets};
use super::*;
use crate::in_game::omsi_world::{Doors, RunRef, ScheduledTraffic};
use opts_bevy::bus_stops::BusStopMarker;
use opts_sim::pedestrian::LaneId;

/// Most scheduled trains at once.
const MAX_RAIL: usize = 6;
/// A train appears at least this far (m) from the camera and at most this far, and not in plain
/// view within `VIEW_CLEAR_M`.
const APPEAR_MIN_M: f32 = 60.0;
const APPEAR_MAX_M: f32 = 450.0;
const VIEW_CLEAR_M: f32 = 200.0;
/// Seconds before its start that a run gets its train.
const LEAD_S: f64 = 90.0;
/// A stop object may stand this far (m) from its track.
const TRACK_REACH_M: f32 = 14.0;
/// Dwell limits (s).
/// A train that comes from upstream of a loaded stop is put this far (m/s times seconds to its
/// arrival) back.
const UPSTREAM_MS: f32 = 15.0;
const DWELL_MIN_S: f32 = 15.0;
const DWELL_MAX_S: f32 = 60.0;

/// Marks the head unit of a scheduled train (the maps draw it).
#[derive(Component)]
pub struct ScheduledTrain;

/// A stop of a train's run.
#[derive(Debug, Clone)]
pub(super) struct RailStop {
    pub(super) station: u32,
    pub(super) lane: LaneId,
    /// Where the head stands when the train is at the stop (m along the lane).
    pub(super) s: f32,
    pub(super) dwell_s: f32,
    /// Earliest departure (clock, s).
    pub(super) depart_not_before: f64,
    /// Scheduled arrival (clock, s).
    pub(super) scheduled: f64,
    pub(super) arrived_at: Option<f64>,
}

/// The run a scheduled train drives and the stops still ahead.
#[derive(Debug, Clone)]
pub(super) struct RailPlan {
    pub(super) run: RunRef,
    pub(super) stops: VecDeque<RailStop>,
    pub(super) retire_at: Option<f64>,
    /// Where the doors are (`AI_Scheduled_AtStation`, as for road vehicles).
    pub(super) doors: Doors,
    /// Seconds since the units' scripts last ran a frame.
    pub(super) since: f32,
}

mod route;
pub(super) use route::*;
mod spawn;
use spawn::*;

/// Spawns the train of the next run that is on the rails at the session clock and has none.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn spawn_scheduled_trains(
    time: Res<Time<Real>>,
    mut commands: Commands,
    job: Option<ResMut<WorldLoadJob>>,
    env: Option<Res<opts_bevy::SessionEnvironment>>,
    net: Res<RailNetwork>,
    pool: Res<RailAiPool>,
    mut sched: ResMut<ScheduledTraffic>,
    markers: Query<(&BusStopMarker, &GlobalTransform)>,
    camera: Query<&GlobalTransform, With<opts_bevy::MainCamera>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut timer: Local<f32>,
    mut tries: Local<HashMap<RunRef, u32>>,
    mut not_rail: Local<HashSet<String>>,
) {
    *timer += time.delta_secs();
    if *timer < 1.0 {
        return;
    }
    *timer = 0.0;
    let (Some(mut job), Some(env), Ok(cam)) = (job, env, camera.single()) else {
        return;
    };
    let net = &net.0;
    if net.lane_count() == 0 || sched.rail_agents() >= MAX_RAIL {
        return;
    }
    let Some(tt) = job.timetables() else { return };
    let stops = markers.iter().map(|(m, g)| {
        let t = g.translation();
        (m.id_code as u32, Vec2::new(t.x, -t.z))
    });
    let pass = Pass::new((net, &tt, env.seconds_of_day()), cam, stops.collect());
    let mut assets: SpawnAssets = (&mut meshes, &mut materials, &mut images);
    let mut seen = [0_u32; 6];
    for (run, schedule) in sched.active_list().to_vec() {
        seen[0] += 1;
        if sched.rail_has(&run) || sched.road_has(&run) || tries.get(&run).is_some_and(|n| *n >= 3)
        {
            continue;
        }
        let Some(rail) = rail_vehicles(&pass, &pool, (&run, &schedule), &mut not_rail) else {
            continue;
        };
        seen[1] += 1;
        let found = match find_start(&pass, &schedule, &mut seen) {
            Ok(found) => found,
            Err(retry) => {
                if retry {
                    *tries.entry(run.clone()).or_default() += 1;
                }
                continue;
            }
        };
        // The vehicle: one of the group's rail vehicles.
        let file = rail[(run.run + run.tour * 7) % rail.len()];
        let args = (&run, &schedule, file);
        match spawn_run(
            &pass,
            (&mut job, &mut commands, &mut assets),
            &mut sched,
            args,
            &found,
        ) {
            Spawned::Retry => *tries.entry(run.clone()).or_default() += 1,
            Spawned::Placed | Spawned::Stop => return,
        }
    }
    tracing::debug!(
        candidates = seen[0],
        rail = seen[1],
        placed = seen[2],
        routed = seen[3],
        in_range = seen[4],
        "scheduled trains: no spawn this pass"
    );
}

mod scripts;
pub use scripts::step_rail_scripts;

#[cfg(test)]
mod tests;
