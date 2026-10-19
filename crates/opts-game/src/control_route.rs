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
fn clock_text(s: f32) -> String {
    let t = (s.max(0.0).round() as u32) % 86_400;
    format!("{:02}:{:02}:{:02}", t / 3600, t / 60 % 60, t % 60)
}

fn describe_schedule(
    s: &opts_core::formats::timetable::ActiveSchedule,
    stop_limit: usize,
) -> Value {
    json!({
        "line_file": s.line_file,
        "tour": s.tour,
        "trip": s.trip,
        "title": s.title,
        "line": s.line,
        "start": clock_text(s.start_s),
        "stop_count": s.stops.len(),
        "stops": s.stops.iter().take(stop_limit).map(|x| json!({
            "station": x.station_id,
            "name": x.name,
            "arrival": clock_text(x.arrival_s),
            "departure": clock_text(x.departure_s),
        })).collect::<Vec<_>>(),
    })
}

fn timetables(world: &mut World, params: &Value) -> Outcome {
    (|| {
        let line = opt_str(params, "line")?.map(str::to_string);
        let tour = opt_u64(params, "tour")?.map(|n| n as usize);
        let limit = opt_u64(params, "limit")?.map_or(200, |n| n as usize);
        let all = opt_bool(params, "all")?.unwrap_or(false);
        let route = player_route(world)?;
        let Some(tt) = route.handle.timetables() else {
            return Ok(json!({ "available": false, "lines": [] }));
        };
        let Some(line) = line else {
            let lines: Vec<Value> = tt
                .lines
                .iter()
                .filter(|(_, l)| all || l.user_allowed)
                .take(limit)
                .map(|(name, l)| {
                    json!({
                        "file": name,
                        "priority": l.priority,
                        "user_allowed": l.user_allowed,
                        "tours": l.tours.len(),
                        "runs": l.tours.iter().map(|t| t.trips.len()).sum::<usize>(),
                    })
                })
                .collect();
            return Ok(json!({ "available": true, "lines": lines }));
        };
        let Some((_, file)) = tt.lines.iter().find(|(n, _)| n.eq_ignore_ascii_case(&line)) else {
            return Err(RpcError::invalid_params(format!("no line file `{line}`")));
        };
        match tour {
            None => Ok(json!({
                "available": true,
                "line": line,
                "tours": file.tours.iter().enumerate().take(limit).map(|(i, t)| json!({
                    "tour": i,
                    "name": t.name,
                    "vehicle_group": t.vehicle_group,
                    "runs": t.trips.len(),
                })).collect::<Vec<_>>(),
            })),
            Some(ti) => {
                let t = file
                    .tours
                    .get(ti)
                    .ok_or_else(|| RpcError::invalid_params(format!("no tour {ti} in `{line}`")))?;
                Ok(json!({
                    "available": true,
                    "line": line,
                    "tour": ti,
                    "name": t.name,
                    "runs": t.trips.iter().enumerate().take(limit).map(|(i, r)| {
                        let trip = tt.trip(&r.trip);
                        json!({
                            "run": i,
                            "trip": r.trip,
                            "title": trip.map(|x| x.title.clone()),
                            "line": trip.map(|x| x.line.clone()),
                            "start": clock_text(r.start_min * 60.0),
                            "stops": trip.map(|x| x.stations.len()),
                        })
                    }).collect::<Vec<_>>(),
                }))
            }
        }
    })()
    .map_or_else(Outcome::err, Outcome::ok)
}

fn set_timetable(world: &mut World, params: &Value) -> Outcome {
    set_timetable_impl(world, params).map_or_else(Outcome::err, Outcome::ok)
}

/// `bus.set_timetable` as a function: the vehicle menu calls it too.
pub(crate) fn set_timetable_impl(world: &mut World, params: &Value) -> Result<Value, RpcError> {
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

fn set_stop(world: &mut World, params: &Value) -> Outcome {
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
            "timetable": route.handle.schedule().map(|s| describe_schedule(&s, 0)),
            "delay_s": route.handle.with(|s| s.delay_s),
        }))
    })()
    .map_or_else(Outcome::err, Outcome::ok)
}

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
