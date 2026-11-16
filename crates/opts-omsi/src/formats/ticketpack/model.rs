//! The data of a ticket system: the passengers' [`Behaviour`] and the [`Ticket`] types.

/// The general behaviour of the passengers (`[ticketpack]`, four numbers).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Behaviour {
    /// Chance that a boarding passenger stamps a ticket they already have (`stamper_prop`).
    pub stamper_prop: f32,
    /// Chance that a boarding passenger buys a ticket (`ticketbuy_prop`).
    pub ticketbuy_prop: f32,
    /// How much the passengers talk (greetings, thanks), 0 to about 2 (`chattiness`).
    pub chattiness: f32,
    /// How much they complain (`whinge_prop`).
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

/// One ticket type (`[ticket]`, seven lines, or `[ticket_2]`, nine).
#[derive(Debug, Clone, PartialEq)]
pub struct Ticket {
    /// Name in the language of the map.
    pub name: String,
    /// Name in English.
    pub name_english: String,
    /// Stops the ticket is valid for (`0`: any number).
    pub max_stations: u32,
    /// Youngest age that may use it.
    pub age_min: u32,
    /// Oldest age that may use it.
    pub age_max: u32,
    /// Price in the currency's main unit.
    pub value: f32,
    /// Text for the ticket printer (`@` is a line break).
    pub display_string: String,
    /// A day ticket: bought the less often the later it is (`[ticket_2]` only).
    pub day_ticket: bool,
    /// Chance weight of buying this ticket (`[ticket_2]`; `1` for `[ticket]`).
    pub probability: f32,
}
