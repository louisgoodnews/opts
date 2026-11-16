//! Control protocol of the vehicle's surface (M-0120, M-0121): `vehicle.surface` answers the
//! player's vehicle's dirt, wetness and last wash. `dirt` and `wet` (0 to 1) set them, `wiped`
//! washes the dirt off; the scripts read the dirt as `Dirt_Norm` (see the script system). Debug
//! only, like `vehicle.fuel`.

use bevy::prelude::*;
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_bevy::{PlayerVehicle, SessionEnvironment, VehicleController};
use opts_cli::protocol::{code, RpcError};
use opts_sim::vehicle::surface::VehicleSurface;
use serde_json::{json, Value};

/// Registers `vehicle.surface`.
pub fn register(registry: &mut ControlRegistry) {
    registry.register("vehicle.surface", surface);
}

/// The parameters of one request.
#[derive(Debug, Default, PartialEq)]
struct Request {
    dirt: Option<f32>,
    wet: Option<f32>,
    wiped: bool,
}

/// A share from 0 to 1 under `key`: `Ok(None)` when absent, `-32602` when it is not one.
fn share(params: &Value, key: &str) -> Result<Option<f32>, RpcError> {
    let Some(v) = params.get(key) else {
        return Ok(None);
    };
    match v.as_f64() {
        Some(x) if (0.0..=1.0).contains(&x) => Ok(Some(x as f32)),
        _ => Err(RpcError::new(
            code::INVALID_PARAMS,
            format!("{key} must be a number from 0 to 1"),
        )),
    }
}

fn parse(params: &Value) -> Result<Request, RpcError> {
    Ok(Request {
        dirt: share(params, "dirt")?,
        wet: share(params, "wet")?,
        wiped: params.get("wiped").and_then(Value::as_bool) == Some(true),
    })
}

/// Applies the request. The wash comes last, so `dirt` with `wiped` ends at 0.
fn apply(surface: &mut VehicleSurface, r: &Request, at: f32) {
    if let Some(d) = r.dirt {
        surface.dirt = d;
    }
    if let Some(w) = r.wet {
        surface.wet = w;
    }
    if r.wiped {
        surface.wash(at);
    }
}

/// The answer: the values now, and the snow cover of the road the weather gives (read only).
#[must_use]
pub fn describe(surface: &VehicleSurface, snow_road: f32) -> Value {
    json!({
        "dirt": surface.dirt,
        "wet": surface.wet,
        "wiped_at": surface.wiped_at,
        "snow": snow_road,
    })
}

/// The session clock's second of day, 0 without a session environment.
fn clock_second(world: &World) -> f32 {
    world
        .get_resource::<SessionEnvironment>()
        .map_or(0.0, |e| e.seconds_of_day() as f32)
}

/// The road's snow cover from the weather, 0 without a session environment.
fn snow_road(world: &World) -> f32 {
    world.get_resource::<SessionEnvironment>().map_or(0.0, |e| {
        opts_bevy::weather::surface_state(e.conditions()).snow_road
    })
}

fn surface(world: &mut World, params: &Value) -> Outcome {
    let request = match parse(params) {
        Ok(r) => r,
        Err(e) => return Outcome::err(e),
    };
    let (at, snow) = (clock_second(world), snow_road(world));
    let mut q = world.query_filtered::<&mut VehicleController, With<PlayerVehicle>>();
    let Some(mut controller) = q.iter_mut(world).next() else {
        return Outcome::err(RpcError::new(
            code::NOT_READY,
            "there is no player vehicle (start a drive first)",
        ));
    };
    apply(&mut controller.sim.surface, &request, at);
    Outcome::ok(describe(&controller.sim.surface, snow))
}

#[cfg(test)]
mod tests;
