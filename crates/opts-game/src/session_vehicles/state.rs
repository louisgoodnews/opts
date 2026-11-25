//! The live state of one session vehicle as data: pose, velocity, driver inputs, gear and the
//! variables of its scripts (which carry doors, lights, switches and whatever else the scripts
//! remember). The session save stores it per vehicle (`session_vehicles` section, version 2) and
//! puts it back after the vehicle has been rebuilt through the normal spawn path.

use bevy::prelude::*;
use opts_bevy::vehicle_script::VehicleScripts;
use opts_bevy::VehicleController;

use super::ops::{abs_to_local, local_to_abs};
use super::registry::{Lifecycle, SessionVehicles};

pub use opts_game_state::session_vehicles::state::VehicleState;

/// Reads the state of the vehicle `entity`; `None` without a simulation.
#[must_use]
pub fn capture(world: &World, entity: Entity) -> Option<VehicleState> {
    let c = world.get::<VehicleController>(entity)?;
    let ch = &c.sim.chassis;
    let i = &c.sim.inputs;
    let (x, y) = local_to_abs(world, ch.position);
    let (script_floats, script_strings) = world
        .get::<VehicleScripts>(entity)
        .and_then(|s| s.engine.lock().ok().map(|e| e.export_variables()))
        .unwrap_or_default();
    Some(VehicleState {
        x,
        y,
        z: f64::from(ch.elevation_z),
        heading_deg: f64::from(ch.heading_rad.to_degrees()),
        velocity: [ch.local_velocity.x, ch.local_velocity.y],
        yaw_rate: ch.yaw_rate_rad_s,
        steering: i.steering(),
        throttle: i.throttle(),
        service_brake: i.service_brake(),
        retarder_stage: i.retarder_stage(),
        parking_brake: i.is_parking_brake_engaged(),
        gear: i.gear(),
        rpm: c.sim.drivetrain.current_rpm,
        gear_index: c.sim.drivetrain.current_gear,
        script_floats,
        script_strings,
    })
}

/// Puts `state` onto the vehicle `entity`. Returns `false` (and changes nothing) while the
/// vehicle's scripts have not run their `{init}` yet: it would overwrite the variables.
pub fn apply(world: &mut World, entity: Entity, state: &VehicleState) -> bool {
    if let Some(scripts) = world.get::<VehicleScripts>(entity) {
        if !scripts.initialised {
            return false;
        }
    }
    let local = abs_to_local(world, state.x, state.y);
    let Some(mut c) = world.get_mut::<VehicleController>(entity) else {
        return false;
    };
    c.sim
        .teleport(local, (state.heading_deg as f32).to_radians());
    let ch = &mut c.sim.chassis;
    ch.elevation_z = state.z as f32;
    ch.local_velocity = Vec2::new(state.velocity[0], state.velocity[1]);
    ch.yaw_rate_rad_s = state.yaw_rate;
    ch.local_acceleration = Vec2::ZERO;
    c.render_prev = None;
    let i = c.inputs_mut();
    i.set_steering(state.steering);
    i.set_throttle(state.throttle);
    i.set_service_brake(state.service_brake);
    i.set_retarder_stage(state.retarder_stage);
    i.set_parking_brake(state.parking_brake);
    i.set_gear(state.gear);
    c.sim.drivetrain.current_rpm = state.rpm;
    c.sim.drivetrain.current_gear = state.gear_index;
    if let Some(scripts) = world.get::<VehicleScripts>(entity) {
        if let Ok(mut engine) = scripts.engine.lock() {
            engine.import_variables(&state.script_floats, &state.script_strings);
        }
    }
    true
}

/// Gives every living vehicle whose saved state is waiting (a restore) its state, as soon as it
/// can take it. Runs each frame while a session is up; does nothing when nothing waits.
///
/// States wait until no vehicle is still spawning: building a vehicle activates it, which parks the
/// one that was active before, and that must not undo a restored state.
pub fn apply_pending(world: &mut World) {
    if world
        .resource::<SessionVehicles>()
        .records()
        .iter()
        .any(|r| r.lifecycle == Lifecycle::Spawning)
    {
        return;
    }
    let waiting: Vec<(u32, Entity, VehicleState)> = world
        .resource::<SessionVehicles>()
        .records()
        .iter()
        .filter(|r| r.lifecycle == Lifecycle::Alive)
        .filter_map(|r| Some((r.id, r.entity?, r.saved_state.clone()?)))
        .collect();
    for (id, entity, state) in waiting {
        if world.get_entity(entity).is_err() || apply(world, entity, &state) {
            if let Some(r) = world.resource_mut::<SessionVehicles>().get_mut(id) {
                r.saved_state = None;
            }
            tracing::info!(id, "saved vehicle state applied");
        }
    }
}
