//! The train of a run: its vehicle file, the scripts of its units, its stop points and the spawn itself.

use super::super::*;
use crate::in_game::omsi_world::rail::ai::{build_train, AiTrain, ScriptCanvases, SpawnAssets};
use crate::in_game::omsi_world::rail::composition::VehicleComposition;
use crate::in_game::omsi_world::rail::spec::{load_spec, load_zug_spec, TrainSpec};
use crate::in_game::omsi_world::{engine_for, Doors, ScheduledTraffic};
use opts_core::formats::timetable::ActiveSchedule;

/// The train spec of a vehicle file.
pub(in super::super) fn load_vehicle(job: &WorldLoadJob, file: &str) -> Option<TrainSpec> {
    let loaded = if file.to_ascii_lowercase().ends_with(".zug") {
        load_zug_spec(job, file)
    } else {
        load_spec(job, file)
    };
    loaded.map(|(spec, _)| spec)
}

/// Every unit with scripts of its own runs them (doors, destination display).
pub(in super::super) fn attach_scripts(
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    spec: &TrainSpec,
    (composition, canvases): (&VehicleComposition, Vec<ScriptCanvases>),
    run: &RunRef,
) {
    for (i, (part, canvas)) in composition.parts().zip(canvases).enumerate() {
        let seed = (u64::from(run.run as u32) << 8) ^ i as u64 ^ 0x5A11;
        if let Some((scripts, route)) = engine_for(
            job,
            &spec.units[i].defs.bus_path,
            canvas,
            Some(run),
            seed,
            &[],
        ) {
            commands.entity(part.entity).insert((scripts, route));
        }
    }
}

/// Stop points: the centre of the train (`length` long) stands at the stop object.
pub(in super::super) fn stop_points(
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

/// How [`spawn_run`] ended.
pub(in super::super) enum Spawned {
    /// The train stands; the pass is done.
    Placed,
    /// The vehicle could not be loaded: count a try for the run.
    Retry,
    /// The train could not be built; the pass is done.
    Stop,
}

/// The train of a run, at lane index `i` and arc length `s` on its route, with its stops.
pub(in super::super) fn spawn_run(
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
