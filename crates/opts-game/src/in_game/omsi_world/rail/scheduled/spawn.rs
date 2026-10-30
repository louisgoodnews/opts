//! The steps of [`spawn_scheduled_trains`]: which runs are candidates, where a run's train starts
//! and which way it drives, whether it may appear, and the train with its stops.

use opts_core::formats::timetable::{ActiveSchedule, Timetables};

use super::*;
use crate::in_game::omsi_world::rail::ai::{build_train, AiTrain, ScriptCanvases, SpawnAssets};
use crate::in_game::omsi_world::rail::composition::VehicleComposition;
use crate::in_game::omsi_world::rail::spec::{load_spec, load_zug_spec, TrainSpec};
use crate::in_game::omsi_world::{engine_for, run_phase, Doors, RunPhase, ScheduledTraffic};
use opts_sim::pedestrian::PedestrianNet;

/// The read-only context of one spawn pass.
pub(super) struct Pass<'a> {
    pub(super) net: &'a PedestrianNet,
    pub(super) tt: &'a Timetables,
    pub(super) now: f64,
    /// Camera position and horizontal view direction (session frame).
    pub(super) centre: Vec3,
    pub(super) view: Vec2,
    /// Stop object positions by station id.
    pub(super) stops: HashMap<u32, Vec2>,
}

/// A run's stop objects on the tracks, with the stop index of each.
pub(super) type Places = Vec<(usize, Place)>;

/// Where a run is and where its train starts: its stops on loaded track, whether the train comes
/// from upstream, the first stop, and the part of the way done (`None` before the start).
pub(super) struct Layout {
    pub(super) places: Places,
    pub(super) upstream: bool,
    pub(super) from_stop: usize,
    pub(super) offset: Option<f64>,
}

/// Where a run's train appears: the run's layout, its route, whether the first place is skipped
/// for stop points, and the lane index and arc length on the route.
pub(super) struct Found {
    pub(super) lay: Layout,
    pub(super) plan: RailRoute,
    pub(super) skip_first: bool,
    pub(super) i: usize,
    pub(super) s: f32,
}

impl<'a> Pass<'a> {
    /// The context from the network, timetables and clock, the camera and the stop positions.
    pub(super) fn new(
        (net, tt, now): (&'a PedestrianNet, &'a Timetables, f64),
        cam: &GlobalTransform,
        stops: HashMap<u32, Vec2>,
    ) -> Self {
        let fwd = cam.forward();
        Self {
            net,
            tt,
            now,
            centre: Vec3::new(cam.translation().x, -cam.translation().z, 0.0),
            view: Vec2::new(fwd.x, -fwd.z).normalize_or_zero(),
            stops,
        }
    }

    /// The place of the stop object of `station` on the tracks.
    fn place(&self, station: u32) -> Option<Place> {
        let p = self.stops.get(&station)?;
        let (lane, s, _) = self.net.nearest(Vec3::new(p.x, p.y, 0.0), TRACK_REACH_M)?;
        Some(Place { lane, s })
    }
}

/// The group's rail vehicles of a run's tour; `None` when the run has no tour or the group has no
/// rail vehicle (remembered in `not_rail`).
pub(super) fn rail_vehicles<'a>(
    pass: &Pass,
    pool: &'a RailAiPool,
    (run, schedule): (&RunRef, &ActiveSchedule),
    not_rail: &mut HashSet<String>,
) -> Option<Vec<&'a String>> {
    let tour = pass.tt.tour(&schedule.line_file, run.tour)?;
    let group = tour.vehicle_group.to_ascii_lowercase();
    if not_rail.contains(&group) {
        return None;
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
        return None;
    }
    Some(rail)
}

/// The stop the run is at or coming from and the part of the way to the next, when it is on the
/// rails now.
pub(super) fn start_phase(schedule: &ActiveSchedule, now: f64) -> Option<(usize, Option<f64>)> {
    Some(match run_phase(schedule, now) {
        RunPhase::NotStarted { wait_s } if wait_s <= LEAD_S => (0, None),
        RunPhase::AtStop { index } => (index, None),
        RunPhase::Between { from, fraction } => (from, Some(fraction)),
        _ => return None,
    })
}

/// Where the run is: at its first stop, between two loaded stops, or on the way to a loaded stop
/// whose predecessor is not on loaded track (the train comes from upstream).
pub(super) fn place_run(
    pass: &Pass,
    schedule: &ActiveSchedule,
    (first, offset): (usize, Option<f64>),
) -> Option<Layout> {
    let place_of = |k: usize| pass.place(schedule.stops.get(k)?.station_id);
    let upstream = matches!(run_phase(schedule, pass.now), RunPhase::Between { .. })
        && place_of(first).is_none()
        && place_of(first + 1).is_some();
    let from_stop = if upstream { first + 1 } else { first };
    let mut places: Places = Vec::new();
    for k in from_stop..schedule.stops.len() {
        match place_of(k) {
            Some(p) => places.push((k, p)),
            None => break,
        }
    }
    if places.is_empty() || (offset.is_some() && !upstream && places.len() < 2) {
        return None;
    }
    Some(Layout {
        places,
        upstream,
        from_stop,
        offset,
    })
}

/// The route over the tracks, where the train starts and whether the first place is the start's
/// neighbour the train drives to (no stop point is made for it); `None` when there is no way.
pub(super) fn route_run(
    pass: &Pass,
    schedule: &ActiveSchedule,
    lay: &Layout,
) -> Option<(RailRoute, Place, bool)> {
    let net = pass.net;
    let targets: Vec<Place> = lay.places.iter().map(|(_, p)| *p).collect();
    // The train starts at `start` (lane, arc length) facing `dir`.
    if lay.upstream {
        let (_, b) = lay.places[0];
        let arrival = f64::from(schedule.stops[lay.from_stop].arrival_s);
        let back = ((arrival - pass.now).max(0.0) as f32 * UPSTREAM_MS).clamp(30.0, APPEAR_MAX_M);
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
        let (r, start) = best?;
        Some((r, start, false))
    } else {
        let start = lay.places[0].1;
        let r = [1_i8, -1]
            .into_iter()
            .filter_map(|d| build_route(net, (start, d), &targets))
            .min_by(|a, b| {
                if better(a, b) {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                }
            })?;
        Some((r, start, lay.offset.is_some()))
    }
}

/// The lane index and arc length on the route where the train appears, when that is in range of
/// the camera and not in plain view.
pub(super) fn appear_at(
    pass: &Pass,
    plan: &RailRoute,
    start: Place,
    (offset, skip_first): (Option<f64>, bool),
) -> Option<(usize, f32)> {
    let net = pass.net;
    let u_start = walk_u(net, &plan.states, 0, start.s);
    let u_here = match offset {
        Some(f) if skip_first => {
            let u_next = plan.stop_u.get(1).copied().unwrap_or(u_start);
            u_start + f as f32 * (u_next - u_start)
        }
        _ => u_start,
    };
    let (i, s) = walk_at(net, &plan.states, u_here)?;
    let l = net.lane(plan.states[i].0)?;
    let at = l.at(s).0;
    let d = Vec2::new(at.x - pass.centre.x, at.y - pass.centre.y);
    let distance = d.length();
    if !(APPEAR_MIN_M..=APPEAR_MAX_M).contains(&distance)
        || (pass.view.dot(d.normalize_or_zero()) > 0.5 && distance < VIEW_CLEAR_M)
    {
        return None;
    }
    Some((i, s))
}

/// The train spec of a vehicle file.
pub(super) fn load_vehicle(job: &WorldLoadJob, file: &str) -> Option<TrainSpec> {
    let loaded = if file.to_ascii_lowercase().ends_with(".zug") {
        load_zug_spec(job, file)
    } else {
        load_spec(job, file)
    };
    loaded.map(|(spec, _)| spec)
}

/// Every unit with scripts of its own runs them (doors, destination display).
pub(super) fn attach_scripts(
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    spec: &TrainSpec,
    (composition, canvases): (&VehicleComposition, Vec<ScriptCanvases>),
    run: &RunRef,
) {
    for (i, (part, canvas)) in composition.parts().zip(canvases).enumerate() {
        let seed = (u64::from(run.run as u32) << 8) ^ i as u64 ^ 0x5A11;
        if let Some((scripts, route)) =
            engine_for(job, &spec.units[i].defs.bus_path, canvas, run, seed, &[])
        {
            commands.entity(part.entity).insert((scripts, route));
        }
    }
}

/// Stop points: the centre of the train (`length` long) stands at the stop object.
pub(super) fn stop_points(
    pass: &Pass,
    schedule: &ActiveSchedule,
    (lay, plan, skip_first): (&Layout, &RailRoute, bool),
    length: f32,
) -> VecDeque<RailStop> {
    let mut stops_ahead = VecDeque::new();
    for (n, (k, p)) in lay.places.iter().enumerate() {
        if n < usize::from(skip_first) {
            continue;
        }
        let st = &schedule.stops[*k];
        let Some(&idx) = plan.stop_index.get(n) else {
            break;
        };
        let (lane, d) = plan.states[idx];
        let len = pass.net.lane(lane).map_or(0.0, |l| l.length());
        let target = (p.s + f32::from(d) * length / 2.0).clamp(0.0, len);
        let dwell = pass
            .tt
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
    stops_ahead
}

/// Where the train of a run appears, counting the steps reached in `seen`; `Err(true)` when the
/// run is to be tried again later (no route), `Err(false)` when it is not to be placed now.
pub(super) fn find_start(
    pass: &Pass,
    schedule: &ActiveSchedule,
    seen: &mut [u32; 6],
) -> Result<Found, bool> {
    let phase = start_phase(schedule, pass.now).ok_or(false)?;
    let lay = place_run(pass, schedule, phase).ok_or(false)?;
    seen[2] += 1;
    let (plan, start, skip_first) = route_run(pass, schedule, &lay).ok_or(true)?;
    seen[3] += 1;
    let (i, s) = appear_at(pass, &plan, start, (lay.offset, skip_first)).ok_or(false)?;
    Ok(Found {
        lay,
        plan,
        skip_first,
        i,
        s,
    })
}

/// How [`spawn_run`] ended.
pub(super) enum Spawned {
    /// The train stands; the pass is done.
    Placed,
    /// The vehicle could not be loaded: count a try for the run.
    Retry,
    /// The train could not be built; the pass is done.
    Stop,
}

/// The train of a run, at lane index `i` and arc length `s` on its route, with its stops.
pub(super) fn spawn_run(
    pass: &Pass,
    (job, commands, assets): (&mut WorldLoadJob, &mut Commands, &mut SpawnAssets),
    sched: &mut ScheduledTraffic,
    (run, schedule, file): (&RunRef, &ActiveSchedule, &str),
    found: &Found,
) -> Spawned {
    let Some(spec) = load_vehicle(job, file) else {
        return Spawned::Retry;
    };
    let length = spec.units.last().map_or(20.0, |u| u.behind + 12.0);
    let Some((head, composition, canvases)) = build_train(&spec, job, commands, assets, true)
    else {
        return Spawned::Stop;
    };
    attach_scripts(job, commands, &spec, (&composition, canvases), run);
    let Found {
        lay,
        plan,
        skip_first,
        i,
        s,
    } = found;
    let (i, s) = (*i, *s);
    let parts = (lay, plan, *skip_first);
    let stops_ahead = stop_points(pass, schedule, parts, length);
    let (lane, dir) = plan.states[i];
    let mut train = Train::new(lane, s, dir);
    train.route = plan.states[i + 1..].iter().map(|(l, _)| *l).collect();
    let speed = if lay.offset.is_some() { 8.0 } else { 0.0 };
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
                run: run.clone(),
                stops: stops_ahead,
                retire_at: None,
                doors: Doors::Away,
                since: 0.0,
            }),
        },
        ScheduledTrain,
    ));
    Spawned::Placed
}
