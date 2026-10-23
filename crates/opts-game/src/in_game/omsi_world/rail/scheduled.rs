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
use crate::in_game::omsi_world::{run_phase, RunPhase, RunRef, ScheduledTraffic};
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
/// Longest way (m) over the tracks between two stops.
const LEG_LIMIT_M: f32 = 25_000.0;
/// Dwell limits (s).
/// A train that comes from upstream of a loaded stop is put this far (m/s times seconds to its
/// arrival) back.
const UPSTREAM_MS: f32 = 15.0;
const DWELL_MIN_S: f32 = 15.0;
const DWELL_MAX_S: f32 = 60.0;
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

/// Marks the head unit of a scheduled train (the maps draw it).
#[derive(Component)]
pub struct ScheduledTrain;

/// A stop of a train's run.
#[derive(Debug, Clone)]
pub(super) struct RailStop {
    station: u32,
    lane: LaneId,
    /// Where the head stands when the train is at the stop (m along the lane).
    s: f32,
    dwell_s: f32,
    /// Earliest departure (clock, s).
    depart_not_before: f64,
    /// Scheduled arrival (clock, s).
    scheduled: f64,
    arrived_at: Option<f64>,
}

/// The run a scheduled train drives and the stops still ahead.
#[derive(Debug, Clone)]
pub(super) struct RailPlan {
    run: RunRef,
    stops: VecDeque<RailStop>,
    retire_at: Option<f64>,
}

/// How far a train is from a place on a lane.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Ahead {
    /// This far (m) along the track.
    At(f32),
    /// Beyond the look-ahead.
    Far,
    /// Passed, or not reachable.
    Gone,
}

fn distance_to(train: &Train, net: &PedestrianNet, lane: LaneId, s: f32) -> Ahead {
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
pub(super) fn control(
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
pub(super) fn retired(t: &AiTrain, net: &PedestrianNet, clock: f64, cam: Option<Vec3>) -> bool {
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
pub(super) fn run_of(t: &AiTrain) -> Option<&RunRef> {
    t.plan.as_ref().map(|p| &p.run)
}

/// A stop object's place on the tracks: lane, arc length, position (session frame).
#[derive(Debug, Clone, Copy)]
struct Place {
    lane: LaneId,
    s: f32,
}

/// The walking coordinate along `states` (lanes with their direction) of arc length `s` on lane
/// number `i`: metres from the start of the first lane in walking direction.
fn walk_u(net: &PedestrianNet, states: &[(LaneId, i8)], i: usize, s: f32) -> f32 {
    let before: f32 = states[..i]
        .iter()
        .map(|(l, _)| net.lane(*l).map_or(0.0, |l| l.length()))
        .sum();
    let len = net.lane(states[i].0).map_or(0.0, |l| l.length());
    before + if states[i].1 > 0 { s } else { len - s }
}

/// The lane index and arc length at walking coordinate `u` along `states`.
fn walk_at(net: &PedestrianNet, states: &[(LaneId, i8)], u: f32) -> Option<(usize, f32)> {
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
        let Some((head, composition)) = build_train(&spec, &mut job, &mut commands, &mut assets)
        else {
            return;
        };
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

/// The tracks a run takes through its stops.
struct RailRoute {
    /// The lanes in running order, each with the direction it is walked in; the first is the
    /// first stop's.
    states: Vec<(LaneId, i8)>,
    /// Index in `states` of the lane of each stop (parallel to the places).
    stop_index: Vec<usize>,
    /// Walking coordinate of each stop object's place.
    stop_u: Vec<f32>,
}

/// Whether route `a` is better than `b`: more stops reached, then the shorter way.
fn better(a: &RailRoute, b: &RailRoute) -> bool {
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
fn build_route(
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

#[cfg(test)]
mod tests;
