//! The passengers of one vehicle unit (M-0101): boarding through open doors, buying and stamping
//! a ticket, taking a seat or a standing place, and getting off.
//!
//! Pure logic on the cabin's path graph, no rendering: the host feeds the door states and the
//! stop events, reads the counts the scripts ask for (`humans_count`, `GetHumanCountOnSeat`,
//! `GetHumanCountOnPathLink`, `PAX_*_Req`) and shows the positions. Specification:
//! `docs/src/simulation/passengers.md`.

mod crowd;
mod destination;
mod fare;
mod sim;
mod state;

// The pure data, graph and random source live in `opts-sim-passenger-model` (ISS-086).
use opts_sim_passenger_model::{graph, rng, spec};

pub use crowd::standing_room;
pub use destination::{admits, pick, Destination, DestinationTable};
pub use fare::{fare_for_ride, Fare, FareRule, FareStop, COUNT_DAY_TICKETS, ZONE_TICKET_INDEX};
pub use graph::CabinGraph;
pub use rng::Rng;
pub use sim::{Doors, Event, Passenger, PassengerSim, Phase, WALK_SPEED_M_S};
pub use spec::{chain, Behaviour, Cabin, Paths, Place, Ticket, TicketBook, Unit};
