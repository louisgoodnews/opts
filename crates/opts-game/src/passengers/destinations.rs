//! The destinations of the stops, built from the map's timetables, and the destination of the bus.
//!
//! A stop's passengers are drawn from the trips that serve it, each weighted by how many runs of
//! the map's tours use the trip (a timetable service count, not measured demand). Depot and
//! service runs (`BF_` trip files) are left out, and so is every stop whose ID has no named record
//! in `Busstops.cfg`: nobody waits there with a trip to board.

use std::collections::HashSet;
use std::sync::Arc;

use opts_bevy::route::{RouteHandle, VehicleRoute};
use opts_core::formats::timetable::Timetables;
use opts_sim::passengers::{Destination, DestinationTable};

use super::rig::PassengerRig;

/// The table of a map's timetables.
#[must_use]
pub fn destination_table(tt: &Timetables) -> DestinationTable {
    let mut table = DestinationTable::default();
    for (_, line) in &tt.lines {
        for tour in &line.tours {
            for run in &tour.trips {
                add_trip(&mut table, tt, &run.trip);
            }
        }
    }
    table
}

/// Adds one run of a tour: its destination at each named stop of its trip, once per stop.
fn add_trip(table: &mut DestinationTable, tt: &Timetables, name: &str) {
    if is_depot_run(name) {
        return;
    }
    let Some(trip) = tt.trip(name) else {
        return;
    };
    let dest = Destination::new(&trip.line, &trip.title);
    let mut seen = HashSet::new();
    for st in &trip.stations {
        let named = tt.stop(st.id).is_some_and(|s| !s.name.is_empty());
        if named && seen.insert(st.id) {
            table.add(u64::from(st.id), dest.clone(), 1);
        }
    }
}

/// Depot and service runs are trip files named `BF_...`.
fn is_depot_run(trip: &str) -> bool {
    trip.get(..3).is_some_and(|p| p.eq_ignore_ascii_case("BF_"))
}

/// The destination of the bus: the trip its run follows, `None` without a timetable run.
#[must_use]
pub fn bus_destination(handle: &RouteHandle) -> Option<Destination> {
    handle.with(|s| {
        s.schedule
            .as_ref()
            .map(|x| Destination::new(&x.line, &x.title))
    })
}

/// Builds the rig's table on first use, from the timetables the vehicle's route has, and returns
/// the destination of the bus.
pub(super) fn sync_destinations(
    rig: &mut PassengerRig,
    route: Option<&VehicleRoute>,
) -> Option<Destination> {
    let handle = route.map(|r| &r.handle);
    if rig.destinations.is_none() {
        rig.destinations = handle
            .and_then(|h| h.timetables())
            .map(|t| Arc::new(destination_table(&t)));
    }
    handle.and_then(bus_destination)
}

/// Draws the destination of a person who waits at the stop the bus stands at (`None` when there is
/// no stop, no table or no trip serves the stop).
pub(super) fn draw_dest(rig: &mut PassengerRig) -> Option<Destination> {
    let table = rig.destinations.clone()?;
    let stop = rig.stop_id?;
    table.draw(stop, &mut rig.rng)
}

#[cfg(test)]
mod tests;
