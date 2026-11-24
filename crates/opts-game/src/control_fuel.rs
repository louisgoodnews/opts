//! Control protocol of the fuel and battery (M-0123): `vehicle.fuel` answers the player's vehicle's
//! fuel model. With `level` (0 to 1) it sets the charge, with `refuel: true` it fills the store.
//! Both serve tests and tools. With `refuel: true` the answer gains `refused` (M-0123).

use bevy::prelude::*;
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_bevy::{PlayerVehicle, VehicleController};
use opts_cli::protocol::{code, RpcError};
use opts_sim::vehicle::fuel::{FuelKind, FuelState};
use opts_sim::VehicleSimulation;
use serde_json::{json, Value};

/// Registers `vehicle.fuel`.
pub fn register(registry: &mut ControlRegistry) {
    registry.register("vehicle.fuel", fuel);
}

fn kind_name(kind: FuelKind) -> &'static str {
    match kind {
        FuelKind::Diesel => "diesel",
        FuelKind::Electric => "electric",
    }
}

/// The answer for one simulation: the level, the content, the capacity, the kind, the mode, the
/// low-level flag and the torque the engine keeps (`model: false` without a model).
#[must_use]
pub fn describe(sim: &VehicleSimulation) -> Value {
    match sim.fuel.as_ref() {
        Some(f) => model_json(f),
        None => json!({ "model": false }),
    }
}

fn model_json(f: &FuelState) -> Value {
    json!({
        "model": true,
        "kind": kind_name(f.kind),
        "capacity": f.capacity,
        "content": f.content,
        "level": f.level(),
        "mode": f.mode.key(),
        "low": f.is_low(),
        "torque_factor": f.torque_factor(),
    })
}

/// The level the request asks for, if any: `Err` when it is not a number from 0 to 1.
fn requested_level(params: &Value) -> Result<Option<f32>, RpcError> {
    let Some(v) = params.get("level") else {
        return Ok(None);
    };
    match v.as_f64() {
        Some(x) if (0.0..=1.0).contains(&x) => Ok(Some(x as f32)),
        _ => Err(RpcError::new(
            code::INVALID_PARAMS,
            "level must be a number from 0 to 1",
        )),
    }
}

/// Applies the request to the simulation and answers with the fuel model. `Err` for a bad request.
/// With `refuel: true` the answer gains `refused`: the refuel was refused (the vehicle moves, or it
/// has no model; M-0123). Without `refuel` the answer is unchanged.
fn apply_request(sim: &mut VehicleSimulation, params: &Value) -> Result<Value, RpcError> {
    let level = requested_level(params)?;
    let refused =
        (params.get("refuel").and_then(Value::as_bool) == Some(true)).then(|| !sim.refuel());
    if let (Some(level), Some(f)) = (level, sim.fuel.as_mut()) {
        f.set_level(level);
    }
    let mut answer = describe(sim);
    if let (Some(refused), Some(map)) = (refused, answer.as_object_mut()) {
        map.insert("refused".into(), json!(refused));
    }
    Ok(answer)
}

fn fuel(world: &mut World, params: &Value) -> Outcome {
    let mut q = world.query_filtered::<&mut VehicleController, With<PlayerVehicle>>();
    let Some(mut controller) = q.iter_mut(world).next() else {
        return Outcome::err(RpcError::new(
            code::NOT_READY,
            "there is no player vehicle (start a drive first)",
        ));
    };
    match apply_request(&mut controller.sim, params) {
        Ok(answer) => Outcome::ok(answer),
        Err(e) => Outcome::err(e),
    }
}
