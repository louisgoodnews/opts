//! Stepping passenger movement, ticket buying, stamping, and seat/standing placement.

use super::*;

mod boarding;
mod walk;

impl PassengerSim {
    /// Advances the passengers by `dt` seconds with the doors as given.
    pub fn step(&mut self, dt: f32, doors: &Doors) {
        let mut gone = Vec::new();
        for i in 0..self.passengers.len() {
            match self.passengers[i].phase {
                Phase::WaitingOutside { entry } => {
                    if doors.entries.get(entry).copied().unwrap_or(false) {
                        self.enter(i, entry);
                    }
                }
                Phase::Walking { to } => self.walk(i, to, dt),
                Phase::Buying { left_s } => {
                    if left_s - dt <= 0.0 {
                        self.finish_buying(i);
                    } else {
                        self.passengers[i].phase = Phase::Buying {
                            left_s: left_s - dt,
                        };
                    }
                }
                Phase::Stamping { left_s } => {
                    if left_s - dt <= 0.0 {
                        let p = &mut self.passengers[i];
                        p.stamped = true;
                        let id = p.id;
                        self.events.push(Event::Stamped { id });
                        self.go_to_place(i);
                    } else {
                        self.passengers[i].phase = Phase::Stamping {
                            left_s: left_s - dt,
                        };
                    }
                }
                Phase::Settled { .. } => {}
                Phase::WaitingToLeave { exit } => {
                    if doors.exits.get(exit).copied().unwrap_or(false) {
                        gone.push((i, exit));
                    }
                }
            }
        }
        for &(i, exit) in gone.iter().rev() {
            let p = self.passengers.remove(i);
            self.events.push(Event::Alighted { id: p.id, exit });
        }
    }

    fn arrived(&mut self, i: usize) {
        match self.passengers[i].next {
            Next::Sale => {
                self.passengers[i].phase = Phase::Buying { left_s: BUY_S };
            }
            Next::Stamp => {
                self.passengers[i].phase = Phase::Stamping { left_s: STAMP_S };
            }
            Next::Place => {
                let id = self.passengers[i].id;
                let place = self.passengers[i].place.unwrap_or(usize::MAX);
                if let Some(pl) = self.cabin.places.get(place) {
                    self.passengers[i].position = pl.position;
                    self.passengers[i].unit = pl.unit;
                    self.events.push(Event::Settled {
                        id,
                        place,
                        seated: pl.is_seat(),
                    });
                }
                self.passengers[i].phase = Phase::Settled { place };
            }
            Next::Exit => {
                let at = self.passengers[i].at;
                let exit = self.exit_nodes.iter().position(|&n| n == at).unwrap_or(0);
                self.passengers[i].phase = Phase::WaitingToLeave { exit };
            }
        }
    }
}
