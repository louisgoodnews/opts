//! The plain data the passenger simulation works on. The game builds it from the OMSI files
//! (`passengercabin.cfg`, `paths.cfg`, the `.otp` ticket system); the simulation itself knows no
//! file format.

use glam::Vec3;

/// A seat or standing place.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Place {
    /// Where the passenger touches the vehicle (segment space: x right, y forward, z up).
    pub position: Vec3,
    /// Seat height above the floor; `0` is a standing place.
    pub seat_height: f32,
    /// Rotation about the vertical in degrees (`0` faces forward).
    pub rotation_deg: f32,
}

impl Place {
    /// A seat (else a standing place).
    #[must_use]
    pub fn is_seat(&self) -> bool {
        self.seat_height > 0.0
    }
}

/// The cabin of a unit: its places and the pathpoints of its doors and machines.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Cabin {
    /// Places in file order (the index space of `GetHumanCountOnSeat`).
    pub places: Vec<Place>,
    /// Boarding pathpoints.
    pub entries: Vec<usize>,
    /// Alighting pathpoints.
    pub exits: Vec<usize>,
    /// Pathpoints of the ticket validators.
    pub stampers: Vec<usize>,
    /// Pathpoints of the ticket sale points.
    pub ticket_sales: Vec<usize>,
    /// The unit sells no tickets.
    pub no_ticket_sale: bool,
}

impl Cabin {
    /// Number of seats.
    #[must_use]
    pub fn seat_count(&self) -> usize {
        self.places.iter().filter(|p| p.is_seat()).count()
    }

    /// Number of standing places.
    #[must_use]
    pub fn standing_count(&self) -> usize {
        self.places.len() - self.seat_count()
    }
}

/// The walkable graph of a unit as given: pathpoints and links.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Paths {
    /// Pathpoint positions (segment space).
    pub points: Vec<Vec3>,
    /// Links between pathpoints (walkable both ways).
    pub links: Vec<(usize, usize)>,
}

/// How the passengers of a map behave.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Behaviour {
    /// Chance that a boarding passenger stamps a ticket they have.
    pub stamper_prop: f32,
    /// Chance that a boarding passenger buys a ticket.
    pub ticketbuy_prop: f32,
    /// How much they talk.
    pub chattiness: f32,
    /// How much they complain.
    pub whinge_prop: f32,
}

impl Default for Behaviour {
    fn default() -> Self {
        Self {
            stamper_prop: 0.2,
            ticketbuy_prop: 0.3,
            chattiness: 1.0,
            whinge_prop: 0.2,
        }
    }
}

/// One ticket type.
#[derive(Debug, Clone, PartialEq)]
pub struct Ticket {
    /// Text for the ticket printer (`@` is a line break).
    pub display: String,
    /// Price in the main currency unit.
    pub value: f32,
    /// Youngest age.
    pub age_min: u32,
    /// Oldest age.
    pub age_max: u32,
    /// Stops it covers (`0`: any number).
    pub max_stations: u32,
    /// Weight of buying this ticket.
    pub probability: f32,
    /// A day ticket.
    pub day_ticket: bool,
}

/// A map's ticket system.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TicketBook {
    /// The behaviour of the passengers.
    pub behaviour: Behaviour,
    /// The tickets in the order scripts and meshes index them.
    pub tickets: Vec<Ticket>,
}

impl TicketBook {
    /// The tickets a passenger of `age` may buy for `stops` stops.
    pub fn eligible(&self, age: u32, stops: u32) -> impl Iterator<Item = (usize, &Ticket)> {
        self.tickets.iter().enumerate().filter(move |(_, t)| {
            age >= t.age_min && age <= t.age_max && (t.max_stations == 0 || stops <= t.max_stations)
        })
    }

    /// The price of ticket `index`.
    #[must_use]
    pub fn price(&self, index: usize) -> Option<f32> {
        self.tickets.get(index).map(|t| t.value)
    }
}
