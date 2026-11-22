//! Where a waiting passenger wants to go: the pair of the trip's line and its display text. A bus
//! with a trip takes a passenger only when the pair of its trip is one the passenger holds; a free
//! drive (no trip) takes any passenger whose destination the stop serves.
//!
//! The weights of a [`DestinationTable`] are timetable service counts (how often a trip is in a
//! tour), not measured demand: no data in the map says who wants to go where.

use std::collections::HashMap;

use super::rng::Rng;

/// A destination: the line of a trip and the destination text it shows.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Destination {
    /// Line text of the trip (`13N`).
    pub line: String,
    /// Destination text of the trip, as the display shows it.
    pub title: String,
}

impl Destination {
    /// The destination of a trip with line text `line` and destination text `title`.
    #[must_use]
    pub fn new(line: &str, title: &str) -> Self {
        Self {
            line: line.to_string(),
            title: title.to_string(),
        }
    }
}

/// The destinations the passengers of each stop are drawn from, with their weights, keyed by the
/// stop's ID code. A stop without an entry serves nothing: nobody there has a trip to board.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DestinationTable {
    by_stop: HashMap<u64, Vec<(Destination, u32)>>,
}

impl DestinationTable {
    /// Adds `weight` to destination `dest` at stop `stop` (a zero weight adds nothing).
    pub fn add(&mut self, stop: u64, dest: Destination, weight: u32) {
        if weight == 0 {
            return;
        }
        let list = self.by_stop.entry(stop).or_default();
        match list.iter_mut().find(|(d, _)| *d == dest) {
            Some((_, w)) => *w += weight,
            None => list.push((dest, weight)),
        }
    }

    /// The destinations of stop `stop` with their weights; empty when the stop serves nothing.
    #[must_use]
    pub fn options(&self, stop: u64) -> &[(Destination, u32)] {
        self.by_stop.get(&stop).map_or(&[], Vec::as_slice)
    }

    /// Draws a destination for a passenger who waits at stop `stop`, each destination as likely
    /// as its weight; `None` when no trip serves the stop.
    pub fn draw(&self, stop: u64, rng: &mut Rng) -> Option<Destination> {
        let options = self.options(stop);
        let total: u64 = options.iter().map(|(_, w)| u64::from(*w)).sum();
        if total == 0 {
            return None;
        }
        let point = ((f64::from(rng.next_f32()) * total as f64) as u64).min(total - 1);
        pick(options, point).cloned()
    }
}

/// The destination at `point` of weighted `options`: the weights are laid end to end in order, so
/// the point `p` in `0..total` falls on the destination whose span holds it. `None` past the end.
#[must_use]
pub fn pick(options: &[(Destination, u32)], point: u64) -> Option<&Destination> {
    let mut end = 0u64;
    for (dest, weight) in options {
        end += u64::from(*weight);
        if point < end {
            return Some(dest);
        }
    }
    None
}

/// Whether a bus whose trip is `bus` takes a passenger with destination `dest`. A bus with a trip
/// takes the passenger only when both destinations are known and equal. A free drive (`bus` is
/// `None`, no trip) takes any passenger whose destination is known: a destination is drawn only
/// from the trips that serve the stop the person waits at (see [`DestinationTable::draw`]), so
/// the stop serves it. A passenger without a destination (no trip serves the stop) never boards.
#[must_use]
pub fn admits(dest: Option<&Destination>, bus: Option<&Destination>) -> bool {
    match (dest, bus) {
        (Some(d), Some(b)) => d == b,
        // Free drive: the bus's line and destination are not matched (decided 2026-11-22).
        (Some(_), None) => true,
        (None, _) => false,
    }
}

#[cfg(test)]
mod tests;
