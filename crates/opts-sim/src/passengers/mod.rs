//! The passengers of one vehicle unit (M-0101): boarding through open doors, buying and stamping
//! a ticket, taking a seat or a standing place, and getting off.
//!
//! Pure logic on the cabin's path graph, no rendering: the host feeds the door states and the
//! stop events, reads the counts the scripts ask for (`humans_count`, `GetHumanCountOnSeat`,
//! `GetHumanCountOnPathLink`, `PAX_*_Req`) and shows the positions. Specification:
//! `docs/src/simulation/passengers.md`.

mod graph;
mod sim;
mod spec;

pub use graph::CabinGraph;
pub use sim::{Doors, Event, Passenger, PassengerSim, Phase, Rng, WALK_SPEED_M_S};
pub use spec::{chain, Behaviour, Cabin, Paths, Place, Ticket, TicketBook, Unit};

#[cfg(test)]
mod tests;
