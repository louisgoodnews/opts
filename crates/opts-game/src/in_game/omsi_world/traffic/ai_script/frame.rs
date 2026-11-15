//! Feeding the scripts of the scheduled vehicles near the camera and running their frame.

use super::*;

/// What the script engine is told about the agent: speed, pedals, the AI flags and the route.
pub(super) fn host_inputs(
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
pub(super) fn run_vehicle(
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
