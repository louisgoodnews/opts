//! The scripts of the units of a scheduled train (doors and display), run near the camera.

use super::*;
use crate::in_game::omsi_world::{next_doors, Doors, DOORS_OPEN_SHARE, FRAME_S, SCRIPT_RADIUS_M};

/// What one train unit's scripts get for this frame.
struct UnitFrame {
    speed: f32,
    doors: Doors,
    atstation: Option<f32>,
    run_frame: bool,
    step: f32,
}

/// Feeds one unit's script engine the train's state and, on a script frame, runs it.
fn run_unit(
    (scripts, route, vars): (
        &mut opts_bevy::VehicleScripts,
        &mut opts_bevy::route::VehicleRoute,
        &opts_bevy::AiVariables,
    ),
    part: Entity,
    frame: &UnitFrame,
) {
    let UnitFrame {
        speed,
        doors,
        atstation,
        run_frame,
        step,
    } = *frame;
    let first = !scripts.initialised;
    scripts.initialised = true;
    let opts_bevy::VehicleScripts { engine, plan, .. } = &mut *scripts;
    let Ok(engine) = engine.get_mut() else {
        return;
    };
    let kmh = speed * 3.6;
    let mut inputs = opts_sim::script::inputs::HostInputs {
        velocity_km_h: kmh,
        velocity_ground_km_h: kmh,
        ai: true,
        ai_engine: 1.0,
        wearlifespan: opts_sim::script::inputs::WEARLIFESPAN_NO_BREAKDOWNS,
        ..Default::default()
    };
    opts_bevy::route::feed_route(&mut inputs, &route.handle);
    inputs.pax_entry_req = [doors == Doors::Open; opts_sim::script::inputs::DOORS];
    inputs.pax_exit_req = [doors == Doors::Open; opts_sim::script::inputs::DOORS];
    if let Some(v) = atstation {
        engine.set("ai_scheduled_atstation", v);
    }
    opts_sim::script::inputs::feed(engine, plan, &inputs);
    for (name, value) in vars.iter() {
        if !engine.writes(name) {
            engine.set(name, value);
        }
    }
    if first {
        engine.run_init();
    }
    if run_frame || first {
        engine.run_frame(step.max(0.01), false);
        let opts_bevy::route::VehicleRoute {
            handle,
            pending_target_s,
            ..
        } = &mut *route;
        opts_bevy::route::keep_target(engine, handle, pending_target_s, step);
        if doors != Doors::Away {
            tracing::debug!(
                unit = ?part,
                door_0 = engine.get("door_0"),
                door_1 = engine.get("door_1"),
                atstation = engine.get("ai_scheduled_atstation"),
                "scheduled train unit"
            );
        }
        for _ in engine.drain_diagnostics() {}
        for _ in engine.drain_events() {}
    }
}

/// Feeds the units' scripts of the scheduled trains near the camera and runs their frame: the
/// doors follow the stop (`AI_Scheduled_AtStation`), the display shows the run's line.
#[allow(clippy::type_complexity)]
pub fn step_rail_scripts(
    time: Res<Time>,
    env: Option<Res<opts_bevy::SessionEnvironment>>,
    camera: Query<&GlobalTransform, With<opts_bevy::MainCamera>>,
    mut trains: Query<(&mut AiTrain, &Transform)>,
    mut units: Query<(
        &mut opts_bevy::VehicleScripts,
        &mut opts_bevy::route::VehicleRoute,
        &opts_bevy::AiVariables,
    )>,
) {
    let (Some(env), Ok(cam)) = (env, camera.single()) else {
        return;
    };
    let clock = env.seconds_of_day();
    let dt = time.delta_secs().min(0.2);
    for (mut t, at) in &mut trains {
        if at.translation.distance(cam.translation()) > SCRIPT_RADIUS_M as f32 {
            continue;
        }
        let speed = t.speed;
        let parts: Vec<Entity> = t.composition.parts().map(|p| p.entity).collect();
        let Some(plan) = t.plan.as_mut() else {
            continue;
        };
        let at_stop = plan
            .stops
            .front()
            .and_then(|s| s.arrived_at.map(|a| (a, f64::from(s.dwell_s))));
        let open = at_stop.is_some_and(|(a, d)| clock - a < d * DOORS_OPEN_SHARE);
        let (doors, atstation) = next_doors(plan.doors, at_stop.is_some(), open);
        plan.doors = doors;
        plan.since += dt;
        let run_frame = plan.since >= FRAME_S;
        let step = if run_frame {
            std::mem::take(&mut plan.since)
        } else {
            0.0
        };
        for part in parts {
            let Ok((mut scripts, mut route, vars)) = units.get_mut(part) else {
                continue;
            };
            run_unit(
                (&mut scripts, &mut route, vars),
                part,
                &UnitFrame {
                    speed,
                    doors,
                    atstation,
                    run_frame,
                    step,
                },
            );
        }
    }
}
