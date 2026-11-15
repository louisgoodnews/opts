//! boarding: passengers taking a place, and where the people inside are counted.

use super::*;

impl PassengerSim {

    /// A passenger of `age` who rides `stops` stops arrives at entry `entry` (clamped to the
    /// entries there are). `None` when the unit has no entry.
    pub fn board(&mut self, age: u32, stops: u32, entry: usize) -> Option<u32> {
        if self.entry_nodes.is_empty() {
            return None;
        }
        let entry = entry.min(self.entry_nodes.len() - 1);
        let node = self.entry_nodes[entry];
        let id = self.next_id;
        self.next_id += 1;
        self.passengers.push(Passenger {
            id,
            age,
            phase: Phase::WaitingOutside { entry },
            ticket: None,
            stamped: false,
            stops_left: stops.max(1),
            position: self.graph.points.get(node).copied().unwrap_or(Vec3::ZERO),
            unit: self.graph.unit(node),
            at: node,
            route: Vec::new(),
            next: Next::Place,
            place: None,
        });
        Some(id)
    }

    /// Passengers inside the unit (not waiting outside).
    #[must_use]
    pub fn humans_inside(&self) -> usize {
        self.passengers
            .iter()
            .filter(|p| !matches!(p.phase, Phase::WaitingOutside { .. }))
            .count()
    }

    /// Passengers inside each of `units` units of the chain, head first (M-0124): the people
    /// aboard counted by the unit they are in; a unit beyond the list holds nobody.
    #[must_use]
    pub fn humans_inside_by_unit(&self, units: usize) -> Vec<usize> {
        let mut counts = vec![0; units];
        for p in &self.passengers {
            if !matches!(p.phase, Phase::WaitingOutside { .. }) {
                if let Some(c) = counts.get_mut(p.unit) {
                    *c += 1;
                }
            }
        }
        counts
    }

    /// Passengers on place `place` (`GetHumanCountOnSeat`).
    #[must_use]
    pub fn on_place(&self, place: usize) -> usize {
        self.passengers
            .iter()
            .filter(|p| p.phase == Phase::Settled { place })
            .count()
    }

    /// Passengers walking along link `link` (`GetHumanCountOnPathLink`).
    #[must_use]
    pub fn on_link(&self, link: usize) -> usize {
        self.passengers
            .iter()
            .filter(|p| {
                matches!(p.phase, Phase::Walking { .. })
                    && p.route
                        .first()
                        .is_some_and(|&to| self.graph.link_between(p.at, to) == Some(link))
            })
            .count()
    }

}
