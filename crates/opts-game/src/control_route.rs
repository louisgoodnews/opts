//! Control protocol of the route (M-0099): `bus.routes` lists the lines and courses of the
//! player's depot, `bus.get_route` says what is selected, `bus.set_route` selects one the way a
//! driver does (the keys of the vehicle's IBIS, the target handed to its displays).

use bevy::prelude::*;
use opts_bevy::control::params::{opt_bool, opt_u64};
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_bevy::route::{select_route_on, RouteError, RouteSelection, VehicleRoute};
use opts_bevy::{PlayerVehicle, VehicleScripts};
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

/// Registers `bus.routes`, `bus.get_route` and `bus.set_route`.
pub fn register(registry: &mut ControlRegistry) {
    registry.register("bus.routes", routes);
    registry.register("bus.get_route", get_route);
    registry.register("bus.set_route", set_route);
}

fn describe(s: &RouteSelection, terminus_text: Option<String>) -> Value {
    json!({
        "route_code": s.route_code,
        "line": s.line,
        "course": s.course,
        "line_string": s.line_string,
        "route_string": s.route_string,
        "terminus_code": s.terminus_code,
        "terminus": terminus_text,
    })
}

fn player_route(world: &mut World) -> Result<VehicleRoute, RpcError> {
    let mut q = world.query_filtered::<&VehicleRoute, With<PlayerVehicle>>();
    q.iter(world).next().cloned().ok_or_else(|| {
        RpcError::new(
            code::NOT_READY,
            "there is no player vehicle with a route (start a drive first)",
        )
    })
}

fn terminus_text(route: &VehicleRoute, s: &RouteSelection) -> Option<String> {
    route.handle.with(|st| {
        let hof = st.hof.as_ref()?;
        let (i, _) = hof.find_terminus_by_code(s.terminus_code)?;
        hof.terminus_string(i, 0).map(str::to_string)
    })
}

fn routes(world: &mut World, params: &Value) -> Outcome {
    (|| {
        let limit = opt_u64(params, "limit")?.map_or(usize::MAX, |n| n as usize);
        let route = player_route(world)?;
        let hof_name = route
            .handle
            .with(|s| s.hof.as_ref().map(|h| h.name.clone()));
        let all = route.handle.routes();
        let listed: Vec<Value> = all
            .iter()
            .take(limit)
            .map(|s| describe(s, terminus_text(&route, s)))
            .collect();
        Ok(json!({
            "depot": hof_name,
            "count": all.len(),
            "routes": listed,
            "selected": route.handle.selection().map(|s| s.route_code),
        }))
    })()
    .map_or_else(Outcome::err, Outcome::ok)
}

fn get_route(world: &mut World, _: &Value) -> Outcome {
    (|| {
        let route = player_route(world)?;
        let selection = route.handle.selection();
        Ok(json!({
            "depot": route.handle.with(|s| s.hof.as_ref().map(|h| h.name.clone())),
            "selected": selection.as_ref().map(|s| describe(s, terminus_text(&route, s))),
            "stop_index": route.handle.with(|s| s.stop_index),
            "typing": route.keys.is_typing(),
        }))
    })()
    .map_or_else(Outcome::err, Outcome::ok)
}

fn set_route(world: &mut World, params: &Value) -> Outcome {
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
        let mut q = world
            .query_filtered::<(&mut VehicleRoute, &VehicleScripts), With<PlayerVehicle>>();
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
    .map_or_else(Outcome::err, Outcome::ok)
}
