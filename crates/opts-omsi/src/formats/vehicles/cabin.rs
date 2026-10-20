//! Parser for the passenger cabin of a vehicle segment (`passengercabin*.cfg`).
//!
//! The cabin lists where passengers sit and stand (`[passpos]`, in file order: the index space of
//! the scripts' `GetHumanCountOnSeat`), the pathpoints (of the segment's [`paths.cfg`](super::paths))
//! where they board and alight, the ticket machines and the cash desk's money and change points.
//! Specification: `docs/src/formats/text/vehicles/passengercabin_cfg.md`.

use glam::Vec3;

use crate::formats::text::{decode_bytes, parse_f64, scan_blocks_with};

/// A place in the cabin (`[passpos]`, `[drivpos]`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CabinPlace {
    /// Where the passenger touches the vehicle (segment-local vehicle space: x right, y forward,
    /// z up): the seat's surface or the floor under the feet.
    pub position: Vec3,
    /// Seat height above the floor; `0` is a standing place.
    pub seat_height: f32,
    /// Rotation about the vertical in degrees (`0` faces forward).
    pub rotation_deg: f32,
}

impl CabinPlace {
    /// A seat (else a standing place).
    #[must_use]
    pub fn is_seat(&self) -> bool {
        self.seat_height > 0.0
    }
}

/// A ticket machine or sale point on a pathpoint (`[stamper]`, `[ticket_sale]`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Machine {
    /// The pathpoint the passenger walks to.
    pub pathpoint: usize,
    /// The slot or output in segment space.
    pub position: Vec3,
}

/// A place where money or change appears (`[ticket_sale_money_point*]`,
/// `[ticket_sale_change_point*]`).
#[derive(Debug, Clone, PartialEq)]
pub struct CashPoint {
    /// Centre in segment space.
    pub position: Vec3,
    /// Spread of the coins in x and y.
    pub variation: (f32, f32),
    /// The mesh that carries it (`_2` variants).
    pub animparent: Option<String>,
}

/// A parsed `passengercabin.cfg`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PassengerCabin {
    /// The driver's place (absent in trailers).
    pub driver: Option<CabinPlace>,
    /// Passenger places in file order.
    pub places: Vec<CabinPlace>,
    /// Boarding pathpoints, one per entry in file order.
    pub entries: Vec<usize>,
    /// Alighting pathpoints.
    pub exits: Vec<usize>,
    /// Pathpoint where the previous unit's graph joins this one.
    pub link_to_prev: Option<usize>,
    /// Pathpoint where the next unit's graph joins this one.
    pub link_to_next: Option<usize>,
    /// Ticket validators.
    pub stampers: Vec<Machine>,
    /// Ticket sale points.
    pub ticket_sales: Vec<Machine>,
    /// Where passengers put money.
    pub money_points: Vec<CashPoint>,
    /// Where the change appears.
    pub change_points: Vec<CashPoint>,
    /// `{noticketsale}`: this unit sells no tickets.
    pub no_ticket_sale: bool,
}

impl PassengerCabin {
    /// Parses the text of a cabin file.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let blocks = scan_blocks_with(text, |k| match k {
            "passpos" | "drivpos" => Some(5),
            "entry" | "exit" | "linktoprevveh" | "linktonextveh" => Some(1),
            "stamper" | "ticket_sale" => Some(4),
            "ticket_sale_money_point" | "ticket_sale_change_point" => Some(5),
            "ticket_sale_money_point_2" | "ticket_sale_change_point_2" => Some(6),
            _ => None,
        });
        let f = |p: &[String], i: usize| {
            p.get(i)
                .and_then(|s| parse_f64(s.trim()).ok())
                .map_or(0.0, |v| v as f32)
        };
        let u = |p: &[String], i: usize| f(p, i).max(0.0).round() as usize;
        let place = |p: &[String]| CabinPlace {
            position: Vec3::new(f(p, 0), f(p, 1), f(p, 2)),
            seat_height: f(p, 3),
            rotation_deg: f(p, 4),
        };
        let machine = |p: &[String]| Machine {
            pathpoint: u(p, 0),
            position: Vec3::new(f(p, 1), f(p, 2), f(p, 3)),
        };
        let cash = |p: &[String]| CashPoint {
            position: Vec3::new(f(p, 0), f(p, 1), f(p, 2)),
            variation: (f(p, 3), f(p, 4)),
            animparent: p
                .get(5)
                .map(|s| s.trim().to_owned())
                .filter(|s| !s.is_empty()),
        };
        let mut c = Self {
            no_ticket_sale: text
                .lines()
                .any(|l| l.trim().eq_ignore_ascii_case("{noticketsale}")),
            ..Self::default()
        };
        for b in &blocks {
            let p = &b.parameters;
            match b.keyword.as_str() {
                "passpos" => c.places.push(place(p)),
                "drivpos" => c.driver = Some(place(p)),
                "entry" => c.entries.push(u(p, 0)),
                "exit" => c.exits.push(u(p, 0)),
                "linktoprevveh" => c.link_to_prev = Some(u(p, 0)),
                "linktonextveh" => c.link_to_next = Some(u(p, 0)),
                "stamper" => c.stampers.push(machine(p)),
                "ticket_sale" => c.ticket_sales.push(machine(p)),
                "ticket_sale_money_point" | "ticket_sale_money_point_2" => {
                    c.money_points.push(cash(p));
                }
                "ticket_sale_change_point" | "ticket_sale_change_point_2" => {
                    c.change_points.push(cash(p));
                }
                _ => {}
            }
        }
        c
    }

    /// Parses a cabin file from raw bytes (CP1252 or UTF-8).
    #[must_use]
    pub fn parse_bytes(bytes: &[u8]) -> Self {
        Self::parse(&decode_bytes(bytes))
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    const CABIN: &str = "{noticketsale}\n[drivpos]\n-0.72\n4.6\n1.1\n0.4\n0\n\n[passpos]\n0.5\n3.0\n0.4\n0.45\n180\n[passpos]\n-0.5\n2.0\n0.4\n0\n0\n[entry]\n3\n[entry]\n0\n[exit]\n23\n[linkToPrevVeh]\n16\n[stamper]\n5\n0.412\n3.178\n1.741\n[ticket_sale]\n1\n-0.064\n5.075\n1.515\n[ticket_sale_money_point_2]\n-0.06\n5.0\n1.5\n0.02\n0.03\nmesh_x\n[ticket_sale_change_point]\n-0.1\n4.9\n1.5\n0.01\n0.01\n";

    #[test]
    fn places_doors_machines_and_cash_points_are_read() {
        let c = PassengerCabin::parse(CABIN);
        assert!(c.no_ticket_sale);
        assert_eq!(
            c.driver.expect("driver").position,
            Vec3::new(-0.72, 4.6, 1.1)
        );
        assert_eq!((c.seat_count(), c.standing_count()), (1, 1));
        assert_eq!(c.places[0].rotation_deg, 180.0);
        assert_eq!((c.entries.clone(), c.exits.clone()), (vec![3, 0], vec![23]));
        assert_eq!(c.link_to_prev, Some(16));
        assert_eq!(c.stampers[0].pathpoint, 5);
        assert_eq!(c.ticket_sales[0].position.y, 5.075);
        assert_eq!(c.money_points[0].animparent.as_deref(), Some("mesh_x"));
        assert_eq!(
            c.change_points[0].animparent, None,
            "the legacy variant has none"
        );
    }

    /// Every installed cabin parses with the doors of its paths (`OMSI_PATH`, `--ignored`).
    #[test]
    #[ignore = "needs an OMSI installation"]
    fn every_installed_cabin_parses() {
        let Ok(root) = std::env::var("OMSI_PATH") else {
            return;
        };
        let (mut files, mut places) = (0, 0);
        for v in std::fs::read_dir(std::path::Path::new(&root).join("Vehicles"))
            .expect("Vehicles")
            .flatten()
        {
            for f in std::fs::read_dir(v.path().join("model"))
                .into_iter()
                .flatten()
                .flatten()
            {
                let name = f.file_name().to_string_lossy().to_lowercase();
                if name.starts_with("passengercabin") && name.ends_with(".cfg") {
                    let c = PassengerCabin::parse_bytes(&std::fs::read(f.path()).expect("read"));
                    files += 1;
                    places += c.places.len();
                }
            }
        }
        eprintln!("{files} cabins, {places} places");
        assert!(files > 100);
    }
}
