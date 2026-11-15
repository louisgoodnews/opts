//! Control protocol of the equipment options (M-0078): `vehicle.setvars` lists the setvars of the
//! vehicle being planned (the wizard) or driven (the session's player vehicle) with kind, range,
//! value and default, and changes several of them at once, all or none.
//!
//! In the wizard a change is the same as in the equipment dialog (the choices of the plan); in a
//! drive it reaches the vehicle's scripts at once and is remembered for a respawn. See
//! `docs/src/formats/text/vehicles/setvars.md`.

use std::collections::HashMap;

use bevy::prelude::*;
use opts_bevy::control::params::opt_bool;
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_cli::protocol::{code, RpcError};
use opts_core::setvars::{Respawn, SetvarCatalogue, SetvarEntry, SetvarKind};
use serde_json::{json, Map, Value};

use crate::in_game::omsi_world::WorldLoadJob;
use crate::new_game_screen::livery::LiveryState;
use crate::new_game_screen::repaints::{vehicle_repaints, VehicleRepaints};
use crate::new_game_screen::setvars_discovery::catalogue_of;
use crate::new_game_screen::types::{GameCatalogue, NewGameSetup};
use crate::session_vehicles::SessionVehicles;

/// Registers `vehicle.setvars`.
pub fn register(registry: &mut ControlRegistry) {
    registry.register("vehicle.setvars", setvars);
}

/// Where the options are read from and written to.

mod check;
mod render;
mod origin;
use check::*;
use render::*;
use origin::*;
pub(crate) use origin::repaints_of_record;
pub use check::apply_live;


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



fn set(
    world: &mut World,
    src: &Source,
    catalogue: &SetvarCatalogue,
    values: &Map<String, Value>,
    allow_unlisted: bool,
) -> Outcome {
    if values.is_empty() {
        return Outcome::err(RpcError::invalid_params(
            "parameter `values` has no options",
        ));
    }
    let batch = match validate(catalogue, values, allow_unlisted) {
        Ok(b) => b,
        Err(bad) => {
            let first = bad[0]["message"].as_str().unwrap_or_default().to_string();
            let text = format!(
                "{} of {} options refused, nothing changed; first: {first}",
                bad.len(),
                values.len()
            );
            return Outcome::err(
                RpcError::invalid_params(text).with_data(json!({ "errors": bad })),
            );
        }
    };
    let mut changed = Vec::new();
    for (name, value) in &batch {
        let previous = catalogue.get(name).map_or(0.0, |e| value_of(src, e));
        let default = catalogue
            .get(name)
            .map_or(0, |e| e.default_for(repaint_of(src)).round() as i32);
        match src.context {
            Context::Wizard => {
                let mut setup = world.resource_mut::<NewGameSetup>();
                if *value == default {
                    setup.custom_setvars.remove(name);
                } else {
                    setup.custom_setvars.insert(name.clone(), *value);
                }
            }
            Context::Drive => apply_live(world, name, *value, default),
        }
        changed.push(json!({ "name": name, "previous": previous, "value": value }));
    }
    Outcome::ok(json!({
        "context": src.context.name(),
        "count": batch.len(),
        "changed": changed,
        "live": src.context == Context::Drive,
    }))
}
