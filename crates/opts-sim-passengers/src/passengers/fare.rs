//! Stop-dependent fares (ISS-053): the price of a ride from a boarding stop to an alighting stop,
//! read from the map's [`TicketBook`].
//!
//! Pure logic, no money: the cash desk (M-0156, not built yet) asks [`fare_for_ride`] for the
//! ticket and its price and takes the payment. [`FareRule::ByStops`] counts the stops ridden and
//! buys the cheapest ticket that covers them (`max_stations`). [`FareRule::ByZones`] counts the
//! zones crossed and maps them to a ticket through [`ZONE_TICKET_INDEX`]. The zone values are
//! placeholders: the Citaro zone logic was read in prose only.

use super::TicketBook;
use tracing::debug;

/// Whether day tickets count as a single ride's fare. A day ticket covers a day, not one ride.
// TODO(owner): confirm that day tickets are never picked for a single ride.
pub const COUNT_DAY_TICKETS: bool = false;

/// Ticket index (in the book's file order) for a ride across 1, 2, 3, ... zones.
// TODO(owner): zone-to-ticket table. The indices are placeholders until the zone logic is settled.
pub const ZONE_TICKET_INDEX: [usize; 3] = [0, 1, 2];

/// A stop of a trip, in running order, with its fare zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FareStop {
    /// Station id (`Busstops.cfg`).
    pub station: u32,
    /// Fare zone of the stop, counted from 0.
    // TODO(owner): zone per station. No zone data is read from the timetable files yet.
    pub zone: u8,
}

/// The rule that sets a ride's price.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FareRule {
    /// The cheapest ticket that covers the stops ridden.
    ByStops,
    /// The ticket for the zones crossed, through [`ZONE_TICKET_INDEX`].
    ByZones,
}

/// The fare of one ride.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fare {
    /// Index of the ticket in the book.
    pub ticket: usize,
    /// Price in the currency's main unit.
    pub price: f32,
    /// Stops ridden after boarding.
    pub stops: u32,
}

/// The fare for riding from `trip[board]` to `trip[alight]`. `None` when the ride is empty or runs
/// backwards, or when no ticket of `age` covers it under `rule`.
#[must_use]
pub fn fare_for_ride(
    book: &TicketBook,
    age: u32,
    trip: &[FareStop],
    board: usize,
    alight: usize,
    rule: FareRule,
) -> Option<Fare> {
    if board >= alight || alight >= trip.len() {
        return None;
    }
    let stops = u32::try_from(alight - board).ok()?;
    let ticket = match rule {
        FareRule::ByStops => cheapest_covering(book, age, stops),
        FareRule::ByZones => {
            let zones = zones_crossed(trip, board, alight);
            zone_ticket(book, age, stops, zones)
        }
    }?;
    let price = book.price(ticket)?;
    debug!(
        board = trip[board].station,
        alight = trip[alight].station,
        fare = price,
        rule = ?rule,
        ticket,
        stops,
        "fare computed"
    );
    Some(Fare {
        ticket,
        price,
        stops,
    })
}

/// The index of the cheapest single-ride ticket that covers `stops` stops.
fn cheapest_covering(book: &TicketBook, age: u32, stops: u32) -> Option<usize> {
    book.eligible(age, stops)
        .filter(|(_, t)| COUNT_DAY_TICKETS || !t.day_ticket)
        .min_by(|(_, a), (_, b)| a.value.total_cmp(&b.value))
        .map(|(i, _)| i)
}

/// The number of zones a ride crosses: the zone difference plus one.
fn zones_crossed(trip: &[FareStop], board: usize, alight: usize) -> usize {
    usize::from(trip[alight].zone.abs_diff(trip[board].zone)) + 1
}

/// The ticket that [`ZONE_TICKET_INDEX`] names for `zones` zones, if `age` may use it for `stops`
/// stops. `None` when the table has no entry for that zone count.
fn zone_ticket(book: &TicketBook, age: u32, stops: u32, zones: usize) -> Option<usize> {
    let wanted = *ZONE_TICKET_INDEX.get(zones.checked_sub(1)?)?;
    book.eligible(age, stops)
        .find(|(i, _)| *i == wanted)
        .map(|(i, _)| i)
}
