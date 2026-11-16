//! Control protocol of the equipment options (M-0078): `vehicle.setvars` lists the setvars of the
//! vehicle being planned (the wizard) or driven (the session's player vehicle) with kind, range,
//! value and default, and changes several of them at once, all or none.
//!
//! In the wizard a change is the same as in the equipment dialog (the choices of the plan); in a
//! drive it reaches the vehicle's scripts at once and is remembered for a respawn. See
//! `docs/src/formats/text/vehicles/setvars.md`.

mod apply;
mod describe;
mod set;
mod source;
mod validate;

pub use apply::apply_live;
pub(crate) use source::repaints_of_record;

use bevy::prelude::*;
use opts_bevy::control::params::opt_bool;
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_cli::protocol::{code, RpcError};
use serde_json::Value;

use crate::new_game_screen::setvars_discovery::catalogue_of;

use describe::get;
use set::set;
use source::source;

/// Registers `vehicle.setvars`.
pub fn register(registry: &mut ControlRegistry) {
    registry.register("vehicle.setvars", setvars);
}

fn setvars(world: &mut World, params: &Value) -> Outcome {
    let Some(src) = source(world) else {
        return Outcome::err(RpcError::new(
            code::NOT_READY,
            "no vehicle to configure: start a drive or choose a vehicle in Plan a drive",
        ));
    };
    let catalogue = catalogue_of(&src.repaints);
    match params.get("values") {
        None | Some(Value::Null) => get(&src, &catalogue),
        Some(Value::Object(values)) => match opt_bool(params, "allow_unlisted") {
            Ok(allow) => set(world, &src, &catalogue, values, allow.unwrap_or(false)),
            Err(e) => Outcome::err(e),
        },
        Some(_) => Outcome::err(RpcError::invalid_params(
            "parameter `values` must be an object of option name to value",
        )),
    }
}
