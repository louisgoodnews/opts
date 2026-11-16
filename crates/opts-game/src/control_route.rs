//! Control protocol of the route (M-0099): `bus.routes` lists the lines and courses of the
//! player's depot, `bus.get_route` says what is selected, `bus.set_route` selects one the way a
//! driver does (the keys of the vehicle's IBIS, the target handed to its displays);
//! `bus.timetables`, `bus.set_timetable` and `bus.set_stop` do the same for the map's timetables.

mod route;
mod timetable_set;
mod timetables;

use bevy::prelude::*;
use opts_bevy::control::state::ControlRegistry;
use opts_bevy::route::VehicleRoute;
use opts_bevy::PlayerVehicle;
use opts_cli::protocol::{code, RpcError};

pub(crate) use route::set_route_impl;
pub(crate) use timetable_set::set_timetable_impl;
use route::{get_route, routes, set_route};
use timetable_set::{set_stop, set_timetable};
use timetables::timetables;

/// Registers `bus.routes`, `bus.get_route` and `bus.set_route`.
pub fn register(registry: &mut ControlRegistry) {
    registry.register("bus.routes", routes);
    registry.register("bus.get_route", get_route);
    registry.register("bus.set_route", set_route);
    registry.register("bus.timetables", timetables);
    registry.register("bus.set_timetable", set_timetable);
    registry.register("bus.set_stop", set_stop);
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
