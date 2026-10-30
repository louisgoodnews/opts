//! The scripts of scheduled AI vehicles (M-0112, the one-script path of M-0103).
//!
//! An unscheduled AI vehicle is a table of animation variables ([`opts_bevy::AiVariables`]). A
//! scheduled bus needs more: its doors open at a stop and its destination display shows the
//! line, and both are done by the vehicle's own scripts (`main_AI.osc`, the door scripts,
//! `VMatrix.osc`). So a scheduled vehicle gets its script engine, run on `{frame_ai}` with the
//! variables OMSI's host gives an AI vehicle (`AI`, `AI_Engine`, `AI_Scheduled_AtStation`, the
//! `PAX_*_Req` flags of passengers who want in or out, `AI_Target_Index` through the trigger
//! `ai_scheduled_settarget`), and the timetable callbacks answer from the run's schedule. The
//! traffic engine still drives the vehicle; the scripts only show it.

use std::path::Path;
use std::sync::{Arc, RwLock};

use opts_bevy::route::{keep_target, RouteHandle, VehicleRoute, TARGET_OFFER_S};
use opts_bevy::VehicleScripts;
use opts_core::formats::script_texture::CpuScriptTexture;
use opts_core::formats::vehicles::BusManifest;
use opts_sim::script::inputs::{feed, HostInputs, WEARLIFESPAN_NO_BREAKDOWNS};

use super::run_schedule::RunRef;
use super::*;

/// Scripts run within this distance (m) of the camera; farther vehicles keep their last state.
pub(in crate::in_game::omsi_world) const SCRIPT_RADIUS_M: f64 = 150.0;
/// Seconds between two runs of the scripts' frame.
pub(in crate::in_game::omsi_world) const FRAME_S: f32 = 0.1;
/// Share of the dwell (from the arrival) in which passengers want to get in and out, so the doors
/// are open for that long and closed before the vehicle leaves.
pub(in crate::in_game::omsi_world) const DOORS_OPEN_SHARE: f64 = 0.5;
/// Passenger doors the request flags are set for.
const DOORS: usize = opts_sim::script::inputs::DOORS;

/// Marks a scheduled AI vehicle whose scripts run, and keeps the time since their last frame.
#[derive(Component, Default)]
pub struct AiScript {
    since: f32,
    doors: Doors,
}

/// Where the doors are in the stop: the host sets `AI_Scheduled_AtStation` to `1` on arrival (the
/// door scripts open), to `-1` when the passengers are through (they close) and the scripts set
/// it back to `0` once the doors are shut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(in crate::in_game::omsi_world) enum Doors {
    #[default]
    Away,
    Open,
    Closing,
}

/// The doors' next phase and the value to write to `AI_Scheduled_AtStation` (`None`: leave it, the
/// scripts own it once they have it): `at_stop` is whether the vehicle stands in a stop, `open`
/// whether the passengers still want in and out.
pub(in crate::in_game::omsi_world) fn next_doors(
    doors: Doors,
    at_stop: bool,
    open: bool,
) -> (Doors, Option<f32>) {
    match (at_stop, doors) {
        (false, Doors::Away) => (Doors::Away, None),
        (false, _) => (Doors::Away, Some(0.0)),
        (true, Doors::Away) => (Doors::Open, Some(1.0)),
        (true, Doors::Open) if !open => (Doors::Closing, Some(-1.0)),
        (true, d) => (d, None),
    }
}

/// Loads the head unit's scripts into an engine, with the depot and the timetable of `run` as
/// callbacks, and puts them on the vehicle `root`. `canvases` are the script textures the model's
/// materials show. Returns whether the vehicle has scripts.
pub(super) fn attach(
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    root: Entity,
    bus_path: &Path,
    (canvases, run): (Arc<RwLock<Vec<CpuScriptTexture>>>, &RunRef),
    (seed, setvars): (u64, &[(String, String)]),
) -> bool {
    let Some((scripts, vehicle_route)) = engine_for(job, bus_path, canvases, run, seed, setvars)
    else {
        return false;
    };
    commands
        .entity(root)
        .insert((scripts, vehicle_route, AiScript::default()));
    true
}

/// The script engine and route of one unit of a scheduled vehicle (a road vehicle's head, a unit
/// of a train), `None` when the unit has no scripts or the run is unknown to its depot.
pub(in crate::in_game::omsi_world) fn engine_for(
    job: &mut WorldLoadJob,
    bus_path: &Path,
    canvases: Arc<RwLock<Vec<CpuScriptTexture>>>,
    run: &RunRef,
    seed: u64,
    setvars: &[(String, String)],
) -> Option<(VehicleScripts, VehicleRoute)> {
    let manifest = std::fs::read(bus_path)
        .ok()
        .and_then(|b| BusManifest::parse_bytes(&b).ok())
        .filter(|m| !m.scripts.is_empty())?;
    let dir = bus_path.parent().map(Path::to_path_buf).unwrap_or_default();
    let (mut engine, _) =
        opts_sim::script::load_vehicle_scripts(&manifest, |rel| job.install.find_in(&dir, rel));
    let folder = job
        .config
        .map_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    let friendly = job.manifest.friendly_name.as_deref().unwrap_or("");
    let hof = opts_bevy::route::find_depot(&dir, job.install.root(), &[friendly, folder]);
    let route = RouteHandle::new(hof.map(Arc::new));
    route.set_timetables(job.timetables());
    if route.select_run(&run.line_file, run.tour, run.run).is_err() {
        return None;
    }
    opts_bevy::route::apply_to_scripts(&mut engine, &route);
    engine.seed_random(seed as u32);
    if let Some(n) = super::super::fleet_number::fleet_number(&job.install, &dir, &manifest, seed) {
        engine.set_string("number", &n);
    }
    engine.set("ai", 1.0);
    engine.set("ai_engine", 1.0);
    // The equipment of the chosen repaint (`[setvar]` of its `.cti`): the model's parts are shown
    // or hidden by these variables, so without them a body can be missing from the vehicle.
    for (var, value) in setvars {
        if let Ok(v) = value.trim().replace(',', ".").parse::<f32>() {
            engine.set(var, v);
        }
    }
    let ctx = opts_bevy::host_callbacks::VehicleHostContext {
        seed,
        route: route.clone(),
        script_textures: canvases,
        fonts: Arc::new(RwLock::new(opts_bevy::text_texture::load_fonts(
            &job.install,
        ))),
        font_bitmaps: opts_bevy::script_text::FontBitmapSource::new(&job.install),
        texture_base_dir: Some(dir.join("Texture")),
        ..Default::default()
    };
    engine.set_external_handler(opts_bevy::host_callbacks::vehicle_callbacks_with_context(
        ctx,
    ));
    let mut vehicle_route = VehicleRoute::new(route);
    vehicle_route.pending_target_s = TARGET_OFFER_S;
    Some((VehicleScripts::new(engine), vehicle_route))
}

/// What the script engine is told about the agent: speed, pedals, the AI flags and the route.
fn host_inputs(
    agent: &opts_traffic::Agent,
    command: opts_traffic::DriverCommand,
    route: &VehicleRoute,
) -> HostInputs {
    let kmh = agent.speed as f32 * 3.6;
    let mut inputs = HostInputs {
        velocity_km_h: kmh,
        velocity_ground_km_h: kmh,
        throttle: command.throttle,
        brake: command.brake,
        ai: true,
        ai_engine: 1.0,
        wearlifespan: WEARLIFESPAN_NO_BREAKDOWNS,
        ..HostInputs::default()
    };
    opts_bevy::route::feed_route(&mut inputs, &route.handle);
    inputs
}

/// Feeds one traffic vehicle's script engine from its agent and runs it on a script frame.
fn run_vehicle(
    tv: &TrafficVehicle,
    (scripts, route, ai, vars): (
        &mut VehicleScripts,
        &mut VehicleRoute,
        &mut AiScript,
        &opts_bevy::AiVariables,
    ),
    (agent, command): (&opts_traffic::Agent, opts_traffic::DriverCommand),
    (clock, dt): (f64, f32),
) {
    let first = !scripts.initialised;
    scripts.initialised = true;
    let VehicleScripts {
        engine,
        plan,
        raised,
        ..
    } = &mut *scripts;
    let Ok(engine) = engine.get_mut() else {
        return;
    };
    let at_stop = agent
        .stops
        .front()
        .and_then(|s| s.arrived_at.map(|a| (a, s.min_dwell_s)));
    let open = at_stop.is_some_and(|(a, d)| clock - a < d * DOORS_OPEN_SHARE);
    let mut inputs = host_inputs(agent, command, route);
    let (doors, atstation) = next_doors(ai.doors, at_stop.is_some(), open);
    ai.doors = doors;
    if let Some(v) = atstation {
        engine.set("ai_scheduled_atstation", v);
    }
    inputs.pax_entry_req = [ai.doors == Doors::Open; DOORS];
    inputs.pax_exit_req = [ai.doors == Doors::Open; DOORS];
    feed(engine, plan, &inputs);
    // The animation inputs the traffic keeps in its table, except what a script writes.
    for (name, value) in vars.iter() {
        if !engine.writes(name) {
            engine.set(name, value);
        }
    }
    if first {
        engine.run_init();
    }
    ai.since += dt;
    if ai.since < FRAME_S && !first {
        return;
    }
    let step = std::mem::take(&mut ai.since);
    engine.run_frame(step, false);
    let VehicleRoute {
        handle,
        pending_target_s,
        ..
    } = &mut *route;
    keep_target(engine, handle, pending_target_s, step);
    if ai.doors == Doors::Open && ai.since < FRAME_S {
        tracing::debug!(
            agent = tv.id.0,
            front = engine.get_string("matrix_schildfrnt"),
            line = engine.get_string("matrix_liniennummerstring"),
            nr = engine.get_string("matrix_nr"),
            ibis = engine.get_string("ibis"),
            target = engine.get("target_index_int"),
            "scheduled vehicle display"
        );
    }
    for _ in engine.drain_diagnostics() {}
    // The events the scripts raise (the door sounds `ev_DoorTrigger…`) wait for the sound layer
    // like a player vehicle's do ([`crate::player_sound`]).
    for e in engine.drain_events() {
        if raised.len() < 64 {
            raised.push(e.name);
        }
    }
}

/// Feeds the scripts of the scheduled vehicles near the camera and runs their frame.
pub fn step_ai_scripts(
    time: Res<Time>,
    state: Res<TrafficState>,
    camera: Query<&GlobalTransform, With<opts_bevy::MainCamera>>,
    mut vehicles: Query<(
        &TrafficVehicle,
        &mut VehicleScripts,
        &mut VehicleRoute,
        &mut AiScript,
        &opts_bevy::AiVariables,
    )>,
) {
    let Ok(cam) = camera.single() else { return };
    let cam = DVec2::new(
        f64::from(cam.translation().x),
        -f64::from(cam.translation().z),
    );
    let dt = time.delta_secs().min(0.2);
    let clock = state.world.clock();
    for (tv, mut scripts, mut route, mut ai, vars) in &mut vehicles {
        if tv.origin.distance(cam) > SCRIPT_RADIUS_M {
            continue;
        }
        let (Some(agent), Some(command)) = (state.world.agent(tv.id), state.world.command(tv.id))
        else {
            continue;
        };
        run_vehicle(
            tv,
            (&mut scripts, &mut route, &mut ai, vars),
            (agent, command),
            (clock, dt),
        );
    }
}

#[cfg(test)]
mod tests;
