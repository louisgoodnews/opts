//! Control-protocol methods of the session vehicles: `session.vehicles`, `session.spawn`,
//! `session.switch`, `session.despawn`, `session.respawn` (`docs/src/architecture/session-vehicles.md`).

mod lifecycle;
mod spawning;

use bevy::prelude::*;
use opts_bevy::control::params::{opt_str, opt_u64};
use opts_bevy::control::state::ControlRegistry;
use opts_cli::protocol::{code, RpcError};
use serde_json::Value;

use crate::in_game::types::SimulationLoadState;
use crate::session_vehicles::ops::OpError;
use crate::session_vehicles::{EntrySel, SessionVehicles, VehicleId};

use lifecycle::{change_map, despawn, reset_road, respawn, switch};
use spawning::{spawn, vehicles};

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
/// The entry point a spawn asks for: `entry_id`, else `entry` (a name), else the session's own.
pub(crate) fn entry_param(params: &Value) -> Result<EntrySel, RpcError> {
    Ok(
        match (opt_u64(params, "entry_id")?, opt_str(params, "entry")?) {
            (Some(i), _) => EntrySel::Id(
                u32::try_from(i).map_err(|_| RpcError::invalid_params("`entry_id` too large"))?,
            ),
            (None, Some(n)) => EntrySel::Name(n.to_owned()),
            (None, None) => EntrySel::Default,
        },
    )
}

/// `stop`: a bus stop's station id, or its name in the map's `Busstops.cfg` (exact, ignoring
/// case, else the first containing it).
fn stop_param(world: &mut World, params: &Value) -> Result<Option<u32>, RpcError> {
    match params.get("stop") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => n
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .map(Some)
            .ok_or_else(|| RpcError::invalid_params("`stop` must be a station id or a name")),
        Some(Value::String(name)) => {
            if let Ok(n) = name.trim().parse::<u32>() {
                return Ok(Some(n));
            }
            let tt = world
                .get_resource_mut::<crate::in_game::omsi_world::WorldLoadJob>()
                .and_then(|mut j| j.timetables())
                .ok_or_else(|| {
                    RpcError::new(code::NOT_FOUND, "the map has no timetable stop list")
                })?;
            let want = name.trim().to_lowercase();
            tt.stops
                .iter()
                .find(|s| s.name.to_lowercase() == want)
                .or_else(|| {
                    tt.stops
                        .iter()
                        .find(|s| s.name.to_lowercase().contains(&want))
                })
                .map(|s| Some(s.id))
                .ok_or_else(|| RpcError::new(code::NOT_FOUND, format!("no bus stop `{name}`")))
        }
        Some(_) => Err(RpcError::invalid_params(
            "`stop` must be a station id or a name",
        )),
    }
}

pub(crate) fn session_ready(world: &World) -> Result<(), RpcError> {
    session_up(world)
}
