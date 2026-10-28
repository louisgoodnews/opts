//! The passengers of one vehicle unit (M-0101): boarding through open doors, buying and stamping
//! a ticket, taking a seat or a standing place, and getting off.
//!
//! Pure logic on the cabin's path graph, no rendering: the host feeds the door states and the
//! stop events, reads the counts the scripts ask for (`humans_count`, `GetHumanCountOnSeat`,
//! `GetHumanCountOnPathLink`, `PAX_*_Req`) and shows the positions. Specification:
//! `docs/src/simulation/passengers.md`.

mod crowd;
mod graph;
mod rng;
mod sim;
mod spec;
mod state;

pub use crowd::standing_room;
pub use graph::CabinGraph;
pub use rng::Rng;
pub use sim::{Doors, Event, Passenger, PassengerSim, Phase, WALK_SPEED_M_S};
pub use spec::{chain, Behaviour, Cabin, Paths, Place, Ticket, TicketBook, Unit};

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_trailer;
