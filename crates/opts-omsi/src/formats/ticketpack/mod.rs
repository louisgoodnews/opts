//! Parser for OMSI ticket system definitions (`TicketPacks/<pack>/*.otp`).
//!
//! An `.otp` defines the general behaviour of the passengers of a map (how many stamp, buy,
//! talk, complain), the ticket types with their prices, and where the voices are. Specification:
//! `docs/src/formats/text/ticketpacks.md`, the keyword table in the `.otp` header itself.

use crate::formats::text::{decode_bytes, parse_f64, scan_blocks_with};

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

/// A parsed ticket system.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TicketPack {
    /// The general behaviour.
    pub behaviour: Behaviour,
    /// The ticket types in file order (the index the scripts and the meshes use).
    pub tickets: Vec<Ticket>,
    /// The folder of the voices, relative to the OMSI root (`[voicepath]`).
    pub voice_path: Option<String>,
}

impl TicketPack {
    /// Parses the text of an `.otp`.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        // Some packs indent their keywords (`\t[ticket]`); a line that is only a keyword counts.
        let text: String = text
            .lines()
            .map(|l| {
                let t = l.trim();
                if t.starts_with('[')
                    && t.ends_with(']')
                    && !t[1..t.len() - 1].contains(['[', ' ', '\t'])
                {
                    t
                } else {
                    l
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        let blocks = scan_blocks_with(&text, |k| match k {
            "ticketpack" => Some(4),
            "ticket" => Some(7),
            "ticket_2" => Some(9),
            "voicepath" => Some(1),
            _ => None,
        });
        let num = |p: &[String], i: usize, default: f32| {
            p.get(i)
                .and_then(|s| parse_f64(s.trim()).ok())
                .map_or(default, |v| v as f32)
        };
        let int = |p: &[String], i: usize| num(p, i, 0.0).max(0.0).round() as u32;
        let mut pack = Self::default();
        for b in blocks {
            let p = &b.parameters;
            match b.keyword.as_str() {
                "ticketpack" => {
                    let d = Behaviour::default();
                    pack.behaviour = Behaviour {
                        stamper_prop: num(p, 0, d.stamper_prop),
                        ticketbuy_prop: num(p, 1, d.ticketbuy_prop),
                        chattiness: num(p, 2, d.chattiness),
                        whinge_prop: num(p, 3, d.whinge_prop),
                    };
                }
                "ticket" | "ticket_2" => {
                    let two = b.keyword == "ticket_2";
                    let text = |i: usize| p.get(i).map(|s| s.trim().to_owned()).unwrap_or_default();
                    pack.tickets.push(Ticket {
                        name: text(0),
                        name_english: text(1),
                        max_stations: int(p, 2),
                        age_min: int(p, 3),
                        age_max: int(p, 4),
                        value: num(p, 5, 0.0),
                        display_string: text(6),
                        day_ticket: two && num(p, 7, 0.0) > 0.5,
                        probability: if two { num(p, 8, 1.0) } else { 1.0 },
                    });
                }
                "voicepath" => {
                    pack.voice_path = p
                        .first()
                        .map(|s| s.trim().to_owned())
                        .filter(|s| !s.is_empty());
                }
                _ => {}
            }
        }
        pack
    }

    /// A small ticket system for maps whose pack is not installed: four tickets in euros.
    #[must_use]
    pub fn fallback() -> Self {
        let t = |name: &str,
                 english: &str,
                 stops: u32,
                 min: u32,
                 max: u32,
                 value: f32,
                 day: bool,
                 p: f32| Ticket {
            name: name.to_owned(),
            name_english: english.to_owned(),
            max_stations: stops,
            age_min: min,
            age_max: max,
            value,
            display_string: name.to_owned(),
            day_ticket: day,
            probability: p,
        };
        Self {
            behaviour: Behaviour::default(),
            tickets: vec![
                t("Kurzstrecke", "Short trip", 4, 15, 200, 1.70, false, 1.0),
                t("Einzelticket", "Single", 0, 15, 200, 2.90, false, 3.0),
                t("Kind", "Child", 0, 6, 14, 1.40, false, 2.0),
                t("Tagesticket", "Day ticket", 0, 15, 200, 7.80, true, 0.4),
            ],
            voice_path: None,
        }
    }

    /// Parses an `.otp` from raw bytes (CP1252 or UTF-8).
    #[must_use]
    pub fn parse_bytes(bytes: &[u8]) -> Self {
        Self::parse(&decode_bytes(bytes))
    }

    /// The tickets a passenger of `age` may buy that cover `stops` stops.
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

#[cfg(test)]
mod tests;
