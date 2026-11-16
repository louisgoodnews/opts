//! Control protocol of the fuel and battery (M-0123): `vehicle.fuel` answers the player's vehicle's
//! fuel model. With `level` (0 to 1) it sets the charge, with `refuel: true` it fills the store.
//! Both serve tests and tools; no key or menu action calls it yet (see M-0123).

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

/// Applies the request to the simulation; `Err` for a bad request.
fn apply_request(sim: &mut VehicleSimulation, params: &Value) -> Result<(), RpcError> {
    let level = requested_level(params)?;
    if params.get("refuel").and_then(Value::as_bool) == Some(true) {
        sim.refuel();
    }
    if let (Some(level), Some(f)) = (level, sim.fuel.as_mut()) {
        f.set_level(level);
    }
    Ok(())
}

fn fuel(world: &mut World, params: &Value) -> Outcome {
    let mut q = world.query_filtered::<&mut VehicleController, With<PlayerVehicle>>();
    let Some(mut controller) = q.iter_mut(world).next() else {
        return Outcome::err(RpcError::new(
            code::NOT_READY,
            "there is no player vehicle (start a drive first)",
        ));
    };
    if let Err(e) = apply_request(&mut controller.sim, params) {
        return Outcome::err(e);
    }
    Outcome::ok(describe(&controller.sim))
}

#[cfg(test)]
mod tests {
    use super::*;
    use opts_sim::vehicle::fuel::FuelMode;

    fn sim_with_fuel() -> VehicleSimulation {
        let mut sim = VehicleSimulation::new_standard_bus(11_500.0, 5.85);
        sim.ensure_fuel(FuelKind::Diesel, 250.0, FuelMode::Realistic);
        sim
    }

    #[test]
    fn a_vehicle_without_a_model_says_so() {
        let sim = VehicleSimulation::new_standard_bus(11_500.0, 5.85);
        assert_eq!(describe(&sim), json!({ "model": false }));
    }

    #[test]
    fn the_level_is_set_and_reported() {
        let mut sim = sim_with_fuel();
        apply_request(&mut sim, &json!({ "level": 0.25 })).unwrap();
        let d = describe(&sim);
        assert!((d["level"].as_f64().unwrap() - 0.25).abs() < 1e-6);
        assert_eq!(d["mode"], "realistic");
        assert_eq!(d["low"], false);
        apply_request(&mut sim, &json!({ "level": 0.05 })).unwrap();
        assert_eq!(describe(&sim)["low"], true);
    }

    #[test]
    fn refuel_fills_the_store_and_a_bad_level_is_refused() {
        let mut sim = sim_with_fuel();
        apply_request(&mut sim, &json!({ "level": 0.0 })).unwrap();
        apply_request(&mut sim, &json!({ "refuel": true })).unwrap();
        assert_eq!(sim.fuel_level(), Some(1.0));
        assert!(apply_request(&mut sim, &json!({ "level": 1.5 })).is_err());
        assert!(apply_request(&mut sim, &json!({ "level": "half" })).is_err());
    }

    #[test]
    fn an_empty_request_changes_nothing() {
        let mut sim = sim_with_fuel();
        apply_request(&mut sim, &json!({})).unwrap();
        assert_eq!(sim.fuel_level(), Some(1.0));
    }
}
