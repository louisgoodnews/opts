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
    /// The unit of the chain the place is in (`0` for a single vehicle).
    pub unit: usize,
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
    /// Per entry: nobody buys who boards through it (missing: may buy).
    pub entry_no_sale: Vec<bool>,
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
    /// Pathpoint positions, each in the space of its own unit.
    pub points: Vec<Vec3>,
    /// Links between pathpoints.
    pub links: Vec<(usize, usize)>,
    /// Per link whether it may be walked from the first point to the second only (missing
    /// entries: both ways).
    pub one_way: Vec<bool>,
    /// The unit of every pathpoint (empty: all in unit 0).
    pub unit_of: Vec<usize>,
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

/// One unit of a chain with the pathpoints where it joins its neighbours.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Unit {
    /// Its cabin.
    pub cabin: Cabin,
    /// Its paths.
    pub paths: Paths,
    /// The pathpoint that joins the unit behind it (`[linkToPrevVeh]` of the file).
    pub link_to_prev: Option<usize>,
    /// The pathpoint that joins the unit in front of it (`[linkToNextVeh]`).
    pub link_to_next: Option<usize>,
}

/// Merges the units of a chain (head first) into one cabin and one graph: indices of pathpoints,
/// links, places, entries and exits run on through the units in order; unit `k`'s
/// `link_to_prev` is joined to unit `k + 1`'s `link_to_next` by a link.
#[must_use]
pub fn chain(units: &[Unit]) -> (Cabin, Paths) {
    let mut cabin = Cabin::default();
    let mut paths = Paths::default();
    let mut offsets = Vec::new();
    for (u, unit) in units.iter().enumerate() {
        let off = paths.points.len();
        offsets.push(off);
        paths.points.extend(unit.paths.points.iter().copied());
        paths
            .unit_of
            .extend(std::iter::repeat_n(u, unit.paths.points.len()));
        paths.one_way.resize(paths.links.len(), false);
        paths
            .links
            .extend(unit.paths.links.iter().map(|&(a, b)| (a + off, b + off)));
        paths.one_way.extend(
            (0..unit.paths.links.len())
                .map(|i| unit.paths.one_way.get(i).copied().unwrap_or(false)),
        );
        cabin
            .places
            .extend(unit.cabin.places.iter().map(|p| Place { unit: u, ..*p }));
        cabin
            .entries
            .extend(unit.cabin.entries.iter().map(|n| n + off));
        cabin.exits.extend(unit.cabin.exits.iter().map(|n| n + off));
        cabin
            .stampers
            .extend(unit.cabin.stampers.iter().map(|n| n + off));
        cabin
            .ticket_sales
            .extend(unit.cabin.ticket_sales.iter().map(|n| n + off));
        let mut no_sale = unit.cabin.entry_no_sale.clone();
        no_sale.resize(unit.cabin.entries.len(), unit.cabin.no_ticket_sale);
        cabin.entry_no_sale.extend(no_sale);
        if u == 0 {
            cabin.no_ticket_sale = unit.cabin.no_ticket_sale;
        }
    }
    for k in 0..units.len().saturating_sub(1) {
        if let (Some(a), Some(b)) = (units[k].link_to_prev, units[k + 1].link_to_next) {
            let (a, b) = (a + offsets[k], b + offsets[k + 1]);
            if a < paths.points.len() && b < paths.points.len() {
                paths.links.push((a, b));
            }
        }
    }
    (cabin, paths)
}
