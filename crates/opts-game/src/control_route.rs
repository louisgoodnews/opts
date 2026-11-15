//! Control protocol of the route (M-0099): `bus.routes` lists the lines and courses of the
//! player's depot, `bus.get_route` says what is selected, `bus.set_route` selects one the way a
//! driver does (the keys of the vehicle's IBIS, the target handed to its displays);
//! `bus.timetables`, `bus.set_timetable` and `bus.set_stop` do the same for the map's timetables.

use bevy::prelude::*;
use opts_bevy::control::params::{opt_bool, opt_str, opt_u64};
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_bevy::route::{select_route_on, select_run_on, RouteError, RouteSelection, VehicleRoute};
use opts_bevy::{PlayerVehicle, VehicleScripts};
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

/// Registers `bus.routes`, `bus.get_route` and `bus.set_route`.
pub fn register(registry: &mut ControlRegistry) {
    registry.register("bus.routes", routes);
    registry.register("bus.get_route", get_route);
    registry.register("bus.set_route", set_route);
    registry.register("bus.timetables", timetables);
    registry.register("bus.set_timetable", set_timetable);
    registry.register("bus.set_stop", set_stop);
}

/// `h:mm:ss` of a time of day in seconds (wrapped at 24 h).

mod read;
mod schedule;
mod timetable;
use read::*;
use schedule::*;
use timetable::*;
pub(crate) use timetable::set_timetable_impl;


fn set_route(world: &mut World, params: &Value) -> Outcome {
    set_route_impl(world, params).map_or_else(Outcome::err, Outcome::ok)
}

/// `bus.set_route` as a function: the vehicle menu calls it too.
pub(crate) fn set_route_impl(world: &mut World, params: &Value) -> Result<Value, RpcError> {
    (|| {
        let clear = opt_bool(params, "clear")?.unwrap_or(false);
        let code_param = opt_u64(params, "route_code")?;
        let line = opt_u64(params, "line")?;
        let course = opt_u64(params, "course")?;
        let wanted = match (code_param, line, course) {
            (Some(c), None, None) => Some(c as i32),
            (None, Some(l), Some(c)) => Some((l * 100 + c) as i32),
            (None, None, None) if clear => None,
            _ => {
                return Err(RpcError::invalid_params(
                    "give `route_code`, or `line` and `course`, or `clear`",
                ))
            }
        };
        let mut q =
            world.query_filtered::<(&mut VehicleRoute, &VehicleScripts), With<PlayerVehicle>>();
        let Some((mut route, scripts)) = q.iter_mut(world).next() else {
            return Err(RpcError::new(
                code::NOT_READY,
                "there is no player vehicle with a route (start a drive first)",
            ));
        };
        let Some(route_code) = wanted else {
            route.handle.clear();
            return Ok(json!({ "selected": null }));
        };
        let mut engine = scripts
            .engine
            .lock()
            .map_err(|_| RpcError::new(code::INTERNAL_ERROR, "the vehicle's scripts are busy"))?;
        match select_route_on(&mut route, &mut engine, route_code) {
            Ok(s) => Ok(json!({
                "selected": describe(&s, None),
                "keypad": true,
                "typing": route.keys.is_typing(),
            })),
            Err(RouteError::NoKeypad) => {
                let s = route.handle.selection();
                Ok(json!({
                    "selected": s.as_ref().map(|s| describe(s, None)),
                    "keypad": false,
                    "typing": false,
                    "note": "the vehicle's scripts have no IBIS keypad of the known kind; the route is selected for the timetable callbacks and the displays that take a scheduled target",
                }))
            }
            Err(e) => Err(RpcError::invalid_params(e.to_string())),
        }
    })()
}
