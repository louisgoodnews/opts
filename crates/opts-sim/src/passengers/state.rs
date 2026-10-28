//! The state of a passenger and what the simulation tells its host: the phases, the doors, the
//! events.

use glam::Vec3;

/// What a passenger does now.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Phase {
    /// Outside, waiting for entry door `entry` to open.
    WaitingOutside {
        /// Index into the cabin's entries.
        entry: usize,
    },
    /// Walking to a pathpoint.
    Walking {
        /// The goal pathpoint.
        to: usize,
    },
    /// Buying a ticket at a sale point.
    Buying {
        /// Seconds left.
        left_s: f32,
    },
    /// Stamping at a validator.
    Stamping {
        /// Seconds left.
        left_s: f32,
    },
    /// Sitting or standing on a place.
    Settled {
        /// Index into the cabin's places.
        place: usize,
    },
    /// At exit `exit`, waiting for its door to open.
    WaitingToLeave {
        /// Index into the cabin's exits.
        exit: usize,
    },
}

/// What the passenger does after the current phase.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Next {
    Sale,
    Stamp,
    Place,
    Exit,
}

/// A passenger.
#[derive(Debug, Clone, PartialEq)]
pub struct Passenger {
    /// Identifier, unique in the simulation.
    pub id: u32,
    /// Age in years (tickets are bought by age).
    pub age: u32,
    /// Where in the plan the passenger is.
    pub phase: Phase,
    /// The ticket held (index in the ticket pack).
    pub ticket: Option<usize>,
    /// The ticket was stamped.
    pub stamped: bool,
    /// Stops to ride before getting off.
    pub stops_left: u32,
    /// Position in the space of the unit the passenger is in.
    pub position: Vec3,
    /// The unit of the chain the passenger is in.
    pub unit: usize,
    /// The pathpoint last reached.
    pub at: usize,
    pub(super) route: Vec<usize>,
    pub(super) next: Next,
    pub(super) place: Option<usize>,
}

/// The state of the doors the passengers use, by entry and exit index of the cabin.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Doors {
    /// Entry doors open.
    pub entries: Vec<bool>,
    /// Exit doors open.
    pub exits: Vec<bool>,
}

/// Something that happened (for the sounds, the scripts and the displays).
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// A passenger came in through entry `entry`.
    Boarded {
        /// Passenger.
        id: u32,
        /// Index into the cabin's entries.
        entry: usize,
    },
    /// A ticket was bought.
    Sold {
        /// Passenger.
        id: u32,
        /// Index in the ticket pack.
        ticket: usize,
        /// Price.
        price: f32,
    },
    /// A ticket was stamped.
    Stamped {
        /// Passenger.
        id: u32,
    },
    /// A passenger took a seat or a standing place.
    Settled {
        /// Passenger.
        id: u32,
        /// Index into the cabin's places.
        place: usize,
        /// A seat (else standing).
        seated: bool,
    },
    /// A passenger got off through exit `exit`.
    Alighted {
        /// Passenger.
        id: u32,
        /// Index into the cabin's exits.
        exit: usize,
    },
}
