//! Selection of a timetable run (`bus.set_timetable`) and of a stop (`bus.set_stop`).

use bevy::prelude::*;
use opts_bevy::control::params::{opt_bool, opt_str, opt_u64};
use opts_bevy::control::state::Outcome;
use opts_bevy::route::{select_run_on, RouteError, VehicleRoute};
use opts_bevy::{PlayerVehicle, VehicleScripts};
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

use super::player_route;
use super::timetables::describe_schedule;

pub(super) fn set_timetable(world: &mut World, params: &Value) -> Outcome {
    set_timetable_impl(world, params).map_or_else(Outcome::err, Outcome::ok)
}

/// `bus.set_timetable` as a function: the vehicle menu and a planned trip call it too. The run
/// the player follows is left out of the scheduled traffic.
pub(crate) fn set_timetable_impl(world: &mut World, params: &Value) -> Result<Value, RpcError> {
    let out = select_timetable(world, params);
    if out.is_ok() {
        let run = (!opt_bool(params, "clear").ok().flatten().unwrap_or(false))
            .then(|| opt_str(params, "line").ok().flatten())
            .flatten()
            .map(|line| crate::in_game::omsi_world::RunRef {
                line_file: line.to_string(),
                tour: opt_u64(params, "tour").ok().flatten().unwrap_or(0) as usize,
                run: opt_u64(params, "run").ok().flatten().unwrap_or(0) as usize,
            });
        if let Some(mut s) =
            world.get_resource_mut::<crate::in_game::omsi_world::ScheduledTraffic>()
        {
            s.player_run = run;
        }
    }
    out
}

fn select_timetable(world: &mut World, params: &Value) -> Result<Value, RpcError> {
    (|| {
        let clear = opt_bool(params, "clear")?.unwrap_or(false);
        let line = opt_str(params, "line")?.map(str::to_string);
        let tour = opt_u64(params, "tour")?.unwrap_or(0) as usize;
        let run = opt_u64(params, "run")?.unwrap_or(0) as usize;
        let stop_limit = opt_u64(params, "stops")?.map_or(5, |n| n as usize);
        let mut q =
            world.query_filtered::<(&mut VehicleRoute, &VehicleScripts), With<PlayerVehicle>>();
        let Some((mut route, scripts)) = q.iter_mut(world).next() else {
            return Err(RpcError::new(
                code::NOT_READY,
                "there is no player vehicle with a route (start a drive first)",
            ));
        };
        if clear {
            route.handle.clear_schedule();
            return Ok(json!({ "schedule": null }));
        }
        let Some(line) = line else {
            return Err(RpcError::invalid_params(
                "give `line` (and `tour`, `run`), or `clear`",
            ));
        };
        let mut engine = scripts
            .engine
            .lock()
            .map_err(|_| RpcError::new(code::INTERNAL_ERROR, "the vehicle's scripts are busy"))?;
        match select_run_on(&mut route, &mut engine, &line, tour, run) {
            Ok((schedule, route_code)) => Ok(json!({
                "schedule": describe_schedule(&schedule, stop_limit),
                "route_code": route_code,
                "typing": route.keys.is_typing(),
            })),
            Err(e @ (RouteError::NoTimetables | RouteError::UnknownRun(_))) => {
                Err(RpcError::invalid_params(e.to_string()))
            }
            Err(e) => Err(RpcError::invalid_params(e.to_string())),
        }
    })()
}

pub(super) fn set_stop(world: &mut World, params: &Value) -> Outcome {
    (|| {
        let index = opt_u64(params, "index")?
            .ok_or_else(|| RpcError::invalid_params("give `index`"))? as usize;
        let route = player_route(world)?;
        let count = route.handle.with(|s| s.tt_stop_count());
        if count > 0 && index >= count {
            return Err(RpcError::invalid_params(format!(
                "stop {index} is beyond the {count} stops of the route"
            )));
        }
        route.handle.set_stop(index);
        Ok(json!({ "stop_index": index, "stop_count": count }))
    })()
    .map_or_else(Outcome::err, Outcome::ok)
}
