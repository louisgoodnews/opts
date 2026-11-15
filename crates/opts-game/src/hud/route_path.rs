//! The player's followed route for the mini map and the large map: the road path from the
//! current stop of the timetable through the stops still ahead, and those stops.
//!
//! Stop positions come from the bus stop objects of the tiles seen so far (remembered when a tile
//! unloads, so the route grows as the drive goes on); the path between two stops follows the
//! lane graph where both are resident and is a straight connector where they are not.

use std::collections::HashMap;

use bevy::math::DVec2;
use bevy::prelude::*;
use opts_bevy::bus_stops::BusStopMarker;
use opts_bevy::route::VehicleRoute;
use opts_bevy::PlayerVehicle;
use opts_traffic::{LaneGraph, LaneId, LaneKind, PathGeometry};

use crate::hud::minimap::MapRoad;
use crate::in_game::omsi_world::TrafficState;

/// Stops ahead that are drawn.
const STOPS_AHEAD: usize = 14;
/// Seconds between two rebuilds of the path.
const REBUILD_S: f32 = 2.0;
/// Reach of a stop to a lane (m) and the heading tolerance, as for scheduled traffic.
const LANE_REACH_M: f64 = 9.0;
const LANE_HEADING_TOL: f64 = 0.65;
/// Width of the drawn path (m); the raster draws it at a fixed pixel width.
const PATH_WIDTH_M: f64 = 4.0;

mod path;
mod scheduled;
mod systems;
pub use path::{path_between, PlayerRoutePath, RouteStop};
pub use scheduled::ScheduledVehicles;
use scheduled::update_scheduled_vehicles;
use systems::update_route_path;

/// Registers the route path.
pub struct RoutePathPlugin;

impl Plugin for RoutePathPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerRoutePath>()
            .init_resource::<ScheduledVehicles>()
            .add_systems(
                Update,
                (update_route_path, update_scheduled_vehicles)
                    .run_if(in_state(crate::app_state::AppState::InGame)),
            );
    }
}

#[cfg(test)]
mod tests;
