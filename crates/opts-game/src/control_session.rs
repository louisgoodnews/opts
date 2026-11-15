//! Control-protocol methods of the session vehicles: `session.vehicles`, `session.spawn`,
//! `session.switch`, `session.despawn`, `session.respawn` (`docs/src/architecture/session-vehicles.md`).


use bevy::prelude::*;
use opts_bevy::control::params::{opt_bool, opt_str, opt_u64};
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

use crate::in_game::types::SimulationLoadState;
use crate::new_game_screen::types::GameCatalogue;
use crate::session_vehicles::ops::{self, OpError};
use crate::session_vehicles::{
    DespawnReason, EntrySel, PlaceLabel, SessionVehicles, SpawnParams, VehicleId,
};

/// Registers the methods.
pub fn register(registry: &mut ControlRegistry) {
    registry.register("session.vehicles", vehicles);
    registry.register("session.spawn", spawn);
    registry.register("session.switch", switch);
    registry.register("session.despawn", despawn);
    registry.register("session.respawn", respawn);
    registry.register("session.reset_road", reset_road);
    registry.register("session.change_map", change_map);
}

fn op_error(e: OpError) -> RpcError {
    match e {
        OpError::NotFound(m) => RpcError::new(code::NOT_FOUND, m),
        OpError::NotReady(m) => RpcError::new(code::NOT_READY, m),
    }
}

fn session_up(world: &World) -> Result<(), RpcError> {
    if !world.contains_resource::<SessionVehicles>() {
        return Err(RpcError::new(
            code::NOT_READY,
            "no session has been started",
        ));
    }
    if world
        .get_resource::<SimulationLoadState>()
        .is_some_and(|s| !s.is_complete)
    {
        return Err(RpcError::new(
            code::NOT_READY,
            "the session is still loading",
        ));
    }
    Ok(())
}

fn id_param(params: &Value) -> Result<Option<VehicleId>, RpcError> {
    opt_u64(params, "id")?
        .map(|n| {
            VehicleId::try_from(n)
                .map_err(|_| RpcError::invalid_params("`id` is not a session vehicle id"))
        })
        .transpose()
}

fn active_or(world: &World, id: Option<VehicleId>) -> Result<VehicleId, RpcError> {
    id.or_else(|| world.resource::<SessionVehicles>().active_id())
        .ok_or_else(|| RpcError::new(code::NOT_READY, "there is no active session vehicle"))
}

mod change_map;
mod spawn;
mod listing;
use change_map::*;
pub(crate) use change_map::{entry_param, session_ready};
use spawn::*;
use listing::*;

fn reset_road(world: &mut World, params: &Value) -> Outcome {
    let result = (|| -> Result<Value, RpcError> {
        session_up(world)?;
        let id = active_or(world, id_param(params)?)?;
        crate::session_vehicles::reset_on_road(world, id).map_err(op_error)?;
        Ok(json!({ "ok": true, "id": id }))
    })();
    result.into()
}
