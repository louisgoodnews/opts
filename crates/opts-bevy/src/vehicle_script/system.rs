//! The script system: the scripted vehicles and the fixed step that advances their scripts.

use super::*;

/// What the script system reads of a vehicle: its simulation, its scripts, whether it is the
/// player's, its route and its passengers.
type ScriptedVehicle<'a> = (
    &'a VehicleController,
    Option<&'a crate::consist_scripts::ScriptConsist>,
    &'a mut VehicleScripts,
    Has<PlayerVehicle>,
    Option<&'a crate::route::VehicleRoute>,
    Option<&'a PassengerFeed>,
);

/// Fixed-step system: initialises and advances the scripts of the player's vehicle and of the
/// vehicles within [`ScriptRadius`] of it (all of them while there is no player's vehicle). A
/// vehicle outside the radius keeps its last state.
pub fn vehicle_script_system(
    time: Res<Time>,
    radius: Option<Res<ScriptRadius>>,
    clock: Option<Res<crate::scenery_runtime::SceneryClock>>,
    mut query: Query<ScriptedVehicle, Without<crate::consist_scripts::ScriptPart>>,
    mut parts: Query<
        (&mut VehicleScripts, Option<&PassengerFeed>),
        With<crate::consist_scripts::ScriptPart>,
    >,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    let radius = radius.map_or(ScriptRadius::default().0, |r| r.0);
    let players: Vec<Vec2> = query
        .iter()
        .filter(|(_, _, _, player, _, _)| *player)
        .map(|(c, _, _, _, _, _)| c.sim.chassis.position)
        .collect();
    for (controller, consist, mut scripts, player, route, pax) in &mut query {
        if !player
            && !players.is_empty()
            && !players
                .iter()
                .any(|p| p.distance(controller.sim.chassis.position) <= radius)
        {
            continue;
        }
        let first = !scripts.initialised;
        scripts.initialised = true;
        let VehicleScripts {
            engine,
            plan,
            raised,
            ..
        } = &mut *scripts;
        let Ok(engine) = engine.get_mut() else {
            continue;
        };
        let mut inputs = host_inputs(controller, clock.as_deref());
        if let Some(r) = route {
            crate::route::feed_route(&mut inputs, &r.handle);
        }
        if let Some(p) = pax {
            inputs.humans_count = p.humans;
            inputs.pax_entry_req = p.entry_req;
            inputs.pax_exit_req = p.exit_req;
        }
        feed(engine, plan, &inputs);
        if first {
            engine.run_init();
        }
        engine.run_frame(dt, false);
        if let Some(consist) = consist {
            crate::consist_scripts::run_parts(consist, &mut parts, engine, &inputs, dt);
        }
        for d in engine.drain_diagnostics() {
            tracing::warn!(file = %d.file, line = d.line, "{}", d.message);
        }
        // The events (sounds, announcements) wait for the sound layer (`opts_game::player_sound`).
        for e in engine.drain_events() {
            tracing::debug!(kind = ?e.kind, name = %e.name, "vehicle script event");
            if raised.len() < 256 {
                raised.push(e.name);
            }
        }
    }
}
