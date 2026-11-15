//! the requests of the stop: who wants to board and who wants to get off.

use super::*;

impl PassengerSim {

    /// Entry doors somebody waits at (`PAX_Entry<n>_Req`).
    #[must_use]
    pub fn entry_requests(&self) -> Vec<bool> {
        let mut r = vec![false; self.entry_nodes.len()];
        for p in &self.passengers {
            if let Phase::WaitingOutside { entry } = p.phase {
                r[entry] = true;
            }
        }
        r
    }

    /// Exit doors somebody waits at (`PAX_Exit<n>_Req`).
    #[must_use]
    pub fn exit_requests(&self) -> Vec<bool> {
        let mut r = vec![false; self.exit_nodes.len()];
        for p in &self.passengers {
            if let Phase::WaitingToLeave { exit } = p.phase {
                r[exit] = true;
            }
        }
        r
    }

    /// The bus reached a stop: every passenger whose ride is over heads for the nearest exit.
    pub fn arrive_at_stop(&mut self) {
        for i in 0..self.passengers.len() {
            let p = &mut self.passengers[i];
            if !matches!(
                p.phase,
                Phase::Settled { .. }
                    | Phase::Walking { .. }
                    | Phase::Buying { .. }
                    | Phase::Stamping { .. }
            ) {
                continue;
            }
            p.stops_left = p.stops_left.saturating_sub(1);
            if p.stops_left == 0 {
                p.place = None;
                p.next = Next::Exit;
                let from = p.at;
                let exit = self.nearest_exit(from);
                let p = &mut self.passengers[i];
                p.route.clear();
                p.phase = Phase::Walking {
                    to: self.exit_nodes.get(exit).copied().unwrap_or(from),
                };
            }
        }
    }

}
