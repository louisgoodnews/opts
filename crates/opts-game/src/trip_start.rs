//! A drive planned with a trip (M-0145): once the player's vehicle and its scripts are there, it
//! follows the trip's run, as `bus.set_timetable` would make it, and the scheduled traffic leaves
//! that run to the player.

use bevy::prelude::*;
use serde_json::json;

use crate::new_game_screen::trips::TripChoice;

/// The trip of the drive being loaded, until the player's vehicle follows it.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct PendingTrip(pub Option<TripChoice>);

/// Fills in the first stop and the texts of a trip that names only its run, from the timetables
/// of the map in `map_dir`. False when the map has no such run or its first stop is no bus stop.
pub fn complete_trip(trip: &mut TripChoice, map_dir: &std::path::Path) -> bool {
    let Some(tt) = opts_bevy::route::find_timetables(map_dir) else {
        return false;
    };
    let Some(s) = tt.schedule(&trip.line_file, trip.tour, trip.run) else {
        return false;
    };
    let Some(first) = s.stops.first().filter(|f| tt.stop(f.station_id).is_some()) else {
        return false;
    };
    trip.station_id = first.station_id;
    trip.stop_name.clone_from(&first.name);
    trip.line = s.line.trim().to_string();
    trip.destination = s.title.trim().to_string();
    trip.depart_s = first.departure_s.max(0.0).round() as u32;
    true
}

/// How many frames to wait for the vehicle's route before giving the trip up.
const MAX_TRIES: u32 = 600;

/// Makes the player's vehicle follow the pending trip as soon as it has a route; the trip is
/// dropped when the drive has no vehicle that can follow it.
fn follow_pending_trip(world: &mut World, mut tries: Local<u32>) {
    let Some(trip) = world
        .get_resource::<PendingTrip>()
        .and_then(|p| p.0.clone())
    else {
        *tries = 0;
        return;
    };
    let params = json!({ "line": trip.line_file, "tour": trip.tour, "run": trip.run });
    match crate::control_route::set_timetable_impl(world, &params) {
        Ok(_) => {
            tracing::info!(
                line = %trip.line_file, tour = trip.tour, run = trip.run, stop = %trip.stop_name,
                "the drive follows the planned trip"
            );
            world.insert_resource(PendingTrip(None));
        }
        Err(e) if e.code == opts_cli::protocol::code::NOT_READY && *tries < MAX_TRIES => {
            *tries += 1;
        }
        Err(e) => {
            tracing::warn!(line = %trip.line_file, "the planned trip cannot be followed: {}", e.message);
            world.insert_resource(PendingTrip(None));
        }
    }
}

/// Registers the system.
pub struct TripStartPlugin;

impl Plugin for TripStartPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingTrip>().add_systems(
            Update,
            follow_pending_trip.run_if(in_state(crate::app_state::AppState::InGame)),
        );
    }
}
