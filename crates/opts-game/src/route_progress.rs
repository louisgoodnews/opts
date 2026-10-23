//! The player's progress along a followed timetable (M-0112): a bus that stands at the stop the
//! timetable names next moves the route on to it, so `GetTTStopIndex` and the delay follow the
//! drive instead of waiting for `route.set_stop`.

use bevy::prelude::*;
use opts_bevy::bus_stops::{nearest_stop, BusStopMarker};
use opts_bevy::route::VehicleRoute;
use opts_bevy::{PlayerVehicle, VehicleController};
use opts_core::formats::timetable::ScheduledStop;

/// How near the stop marker the bus has to stand (m).
const AT_STOP_M: f32 = 18.0;
/// Below this speed the bus counts as standing (km/h).
const STANDING_KMH: f32 = 1.0;
/// How many stops ahead of the current one a stop may be reached (skipped stops).
const LOOKAHEAD: usize = 3;

/// The index of the stop with `station` that the run has reached, given the current `index`:
/// the first stop (nothing reached yet) or one of the next three stops. `None` when the
/// station is not due or is the current stop.
#[must_use]
pub fn reached_stop(stops: &[ScheduledStop], index: usize, station: u64) -> Option<usize> {
    let from = if index == 0 { 0 } else { index + 1 };
    (from..stops.len().min(index + 1 + LOOKAHEAD))
        .find(|&i| u64::from(stops[i].station_id) == station)
        .filter(|&i| i != index)
}

fn follow_schedule(
    player: Query<(&GlobalTransform, &VehicleController, &VehicleRoute), With<PlayerVehicle>>,
    stops: Query<(Entity, &GlobalTransform), With<BusStopMarker>>,
    markers: Query<&BusStopMarker>,
) {
    for (gt, controller, route) in &player {
        if controller.forward_speed_km_h().abs() > STANDING_KMH {
            continue;
        }
        let Some((index, ids)) = route.handle.with(|s| {
            s.schedule
                .as_ref()
                .map(|sc| (s.stop_index, sc.stops.clone()))
        }) else {
            continue;
        };
        let near: Vec<(Entity, Transform)> = stops
            .iter()
            .map(|(e, t)| (e, t.compute_transform()))
            .collect();
        let Some((marker, _)) = nearest_stop(
            near.iter().map(|(e, t)| (*e, t)),
            gt.translation(),
            AT_STOP_M,
        ) else {
            continue;
        };
        let Ok(marker) = markers.get(marker) else {
            continue;
        };
        if let Some(i) = reached_stop(&ids, index, marker.id_code) {
            route.handle.set_stop(i);
        }
    }
}

/// Registers the schedule follower.
pub struct RouteProgressPlugin;

impl Plugin for RouteProgressPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            follow_schedule.run_if(in_state(crate::app_state::AppState::InGame)),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stops(ids: &[u32]) -> Vec<ScheduledStop> {
        ids.iter()
            .map(|&station_id| ScheduledStop {
                station_id,
                name: String::new(),
                arrival_s: 0.0,
                departure_s: 0.0,
                distance_m: None,
            })
            .collect()
    }

    #[test]
    fn the_first_stop_is_reached_from_the_start() {
        assert_eq!(reached_stop(&stops(&[7, 8, 9]), 0, 7), None);
        assert_eq!(reached_stop(&stops(&[7, 8, 9]), 0, 8), Some(1));
    }

    #[test]
    fn the_next_stops_count_and_the_current_one_does_not() {
        let s = stops(&[1, 2, 3, 4, 5, 6]);
        assert_eq!(reached_stop(&s, 1, 2), None);
        assert_eq!(reached_stop(&s, 1, 3), Some(2));
        assert_eq!(reached_stop(&s, 1, 5), Some(4));
        assert_eq!(reached_stop(&s, 1, 6), None, "too far ahead");
        assert_eq!(reached_stop(&s, 4, 1), None, "behind");
    }
}
