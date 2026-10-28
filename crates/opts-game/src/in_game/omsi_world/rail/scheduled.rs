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

use super::ai::{build_train, AiTrain, SpawnAssets};
use super::spec::{load_spec, load_zug_spec};
use super::*;
use crate::in_game::omsi_world::{
    engine_for, run_phase, Doors, RunPhase, RunRef, ScheduledTraffic,
};
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
    let now = env.seconds_of_day();
    let centre = Vec3::new(cam.translation().x, -cam.translation().z, 0.0);
    let fwd = cam.forward();
    let view = Vec2::new(fwd.x, -fwd.z).normalize_or_zero();
    let stops: HashMap<u32, Vec2> = markers
        .iter()
        .map(|(m, g)| {
            let t = g.translation();
            (m.id_code as u32, Vec2::new(t.x, -t.z))
        })
        .collect();
    let place = |station: u32| -> Option<Place> {
        let p = stops.get(&station)?;
        let (lane, s, _) = net.nearest(Vec3::new(p.x, p.y, 0.0), TRACK_REACH_M)?;
        Some(Place { lane, s })
    };
    let mut seen = [0_u32; 6];
    for (run, schedule) in sched.active_list().to_vec() {
        seen[0] += 1;
        if sched.rail_has(&run) || sched.road_has(&run) || tries.get(&run).is_some_and(|n| *n >= 3)
        {
            continue;
        }
        let Some(tour) = tt.tour(&schedule.line_file, run.tour) else {
            continue;
        };
        let group = tour.vehicle_group.to_ascii_lowercase();
        if not_rail.contains(&group) {
            continue;
        }
        let rail: Vec<&String> = pool
            .group_vehicles
            .get(&group)
            .map(|v| {
                v.iter()
                    .filter(|f| pool.rail_files.contains(&f.to_ascii_lowercase()))
                    .collect()
            })
            .unwrap_or_default();
        if rail.is_empty() {
            not_rail.insert(group);
            continue;
        }
        seen[1] += 1;
        let (first, offset) = match run_phase(&schedule, now) {
            RunPhase::NotStarted { wait_s } if wait_s <= LEAD_S => (0, None),
            RunPhase::AtStop { index } => (index, None),
            RunPhase::Between { from, fraction } => (from, Some(fraction)),
            _ => continue,
        };
        // Where the run is: at its first stop, between two loaded stops, or on the way to a
        // loaded stop whose predecessor is not on loaded track (the train comes from upstream).
        let place_of = |k: usize| place(schedule.stops.get(k)?.station_id);
        let upstream = matches!(run_phase(&schedule, now), RunPhase::Between { .. })
            && place_of(first).is_none()
            && place_of(first + 1).is_some();
        let from_stop = if upstream { first + 1 } else { first };
        let mut places: Vec<(usize, Place)> = Vec::new();
        for k in from_stop..schedule.stops.len() {
            match place_of(k) {
                Some(p) => places.push((k, p)),
                None => break,
            }
        }
        if places.is_empty() || (offset.is_some() && !upstream && places.len() < 2) {
            continue;
        }
        seen[2] += 1;
        let targets: Vec<Place> = places.iter().map(|(_, p)| *p).collect();
        // The train starts at `start` (lane, arc length) facing `dir`; `skip_first` when the first
        // place is the start's neighbour the train drives to (no stop point is made for it).
        let (route, start, skip_first) = if upstream {
            let (_, b) = places[0];
            let arrival = f64::from(schedule.stops[from_stop].arrival_s);
            let back = ((arrival - now).max(0.0) as f32 * UPSTREAM_MS).clamp(30.0, APPEAR_MAX_M);
            let mut best: Option<(RailRoute, Place)> = None;
            for dir in [1_i8, -1] {
                let mut t = Train::new(b.lane, b.s, dir);
                if t.retreat(back, net) != Moved::Full {
                    continue;
                }
                let start = Place {
                    lane: t.lane,
                    s: t.s,
                };
                let Some(r) = build_route(net, (start, t.dir), &targets) else {
                    continue;
                };
                if best.as_ref().is_none_or(|(o, _)| better(&r, o)) {
                    best = Some((r, start));
                }
            }
            let Some((r, start)) = best else {
                *tries.entry(run.clone()).or_default() += 1;
                continue;
            };
            (r, start, false)
        } else {
            let start = places[0].1;
            let Some(r) = [1_i8, -1]
                .into_iter()
                .filter_map(|d| build_route(net, (start, d), &targets))
                .min_by(|a, b| {
                    if better(a, b) {
                        std::cmp::Ordering::Less
                    } else {
                        std::cmp::Ordering::Greater
                    }
                })
            else {
                *tries.entry(run.clone()).or_default() += 1;
                continue;
            };
            (r, start, offset.is_some())
        };
        let plan = route;
        seen[3] += 1;
        let u_start = walk_u(net, &plan.states, 0, start.s);
        let u_here = match offset {
            Some(f) if skip_first => {
                let u_next = plan.stop_u.get(1).copied().unwrap_or(u_start);
                u_start + f as f32 * (u_next - u_start)
            }
            _ => u_start,
        };
        let Some((i, s)) = walk_at(net, &plan.states, u_here) else {
            continue;
        };
        let (lane, dir) = plan.states[i];
        let Some(l) = net.lane(lane) else { continue };
        let at = l.at(s).0;
        let d = Vec2::new(at.x - centre.x, at.y - centre.y);
        let distance = d.length();
        if !(APPEAR_MIN_M..=APPEAR_MAX_M).contains(&distance)
            || (view.dot(d.normalize_or_zero()) > 0.5 && distance < VIEW_CLEAR_M)
        {
            continue;
        }
        seen[4] += 1;
        // The vehicle: one of the group's rail vehicles.
        let pick = (run.run + run.tour * 7) % rail.len();
        let file = rail[pick].clone();
        let loaded = if file.to_ascii_lowercase().ends_with(".zug") {
            load_zug_spec(&job, &file)
        } else {
            load_spec(&job, &file)
        };
        let Some((spec, _)) = loaded else {
            *tries.entry(run.clone()).or_default() += 1;
            continue;
        };
        let length = spec.units.last().map_or(20.0, |u| u.behind + 12.0);
        let mut assets: SpawnAssets = (&mut meshes, &mut materials, &mut images);
        let Some((head, composition, canvases)) =
            build_train(&spec, &mut job, &mut commands, &mut assets, true)
        else {
            return;
        };
        // Every unit with scripts of its own runs them (doors, destination display).
        for (i, (part, canvas)) in composition.parts().zip(canvases).enumerate() {
            let seed = (u64::from(run.run as u32) << 8) ^ i as u64 ^ 0x5A11;
            if let Some((scripts, route)) = engine_for(
                &mut job,
                &spec.units[i].defs.bus_path,
                canvas,
                &run,
                seed,
                &[],
            ) {
                commands.entity(part.entity).insert((scripts, route));
            }
        }
        // Stop points: the centre of the train stands at the stop object.
        let mut stops_ahead = VecDeque::new();
        for (n, (k, p)) in places.iter().enumerate() {
            if n < usize::from(skip_first) {
                continue;
            }
            let st = &schedule.stops[*k];
            let Some(&idx) = plan.stop_index.get(n) else {
                break;
            };
            let (lane, d) = plan.states[idx];
            let len = net.lane(lane).map_or(0.0, |l| l.length());
            let target = (p.s + f32::from(d) * length / 2.0).clamp(0.0, len);
            let dwell = tt
                .stop(st.station_id)
                .map_or(DWELL_MIN_S, |s| s.dwell_s)
                .clamp(DWELL_MIN_S, DWELL_MAX_S);
            stops_ahead.push_back(RailStop {
                station: st.station_id,
                lane,
                s: target,
                dwell_s: dwell,
                depart_not_before: f64::from(st.departure_s),
                scheduled: f64::from(st.arrival_s),
                arrived_at: None,
            });
        }
        let mut train = Train::new(lane, s, dir);
        train.route = plan.states[i + 1..].iter().map(|(l, _)| *l).collect();
        let speed = if offset.is_some() { 8.0 } else { 0.0 };
        tracing::info!(
            vehicle = %spec.name,
            line = %schedule.line,
            to = %schedule.title,
            stops = stops_ahead.len(),
            "scheduled train spawned"
        );
        sched.rail_started(run.clone());
        commands.entity(head).insert((
            AiTrain {
                bike: false,
                train,
                speed,
                composition,
                wheel_angle: 0.0,
                plan: Some(RailPlan {
                    run,
                    stops: stops_ahead,
                    retire_at: None,
                    doors: Doors::Away,
                    since: 0.0,
                }),
            },
            ScheduledTrain,
        ));
        return;
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
