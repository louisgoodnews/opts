//! Standing room beyond what a vehicle's `passengercabin.cfg` lists.
//!
//! The cabins list the places a vehicle's scripts count (`GetHumanCountOnSeat`); many list few
//! standing places or none (115 of 644 cabins of the installation list none), while a real bus or
//! tram carries a crowd in its aisles. [`standing_room`] adds standing places along the walkable
//! links of every unit: one place every [`SPACING_M`], in two columns [`COLUMN_OFFSET_M`] either
//! side of the link where the aisle is wide enough and in a single file on the link where it is
//! not, at least [`SEAT_CLEARANCE_M`] from a seat's centre and [`DOOR_CLEARANCE_M`] from a door
//! (the doors stay free). Links that climb (stairs, steeper than [`MAX_SLOPE`]) carry nobody. The
//! places come after the listed ones (their indices are the scripts' index space).

use super::graph::CabinGraph;
use super::spec::{Cabin, Place};
use glam::Vec3;

/// Distance between two standing people along a link (m).
pub const SPACING_M: f32 = 0.45;
/// Offset of the two columns from the link (m).
pub const COLUMN_OFFSET_M: f32 = 0.3;
/// Closest a standing place lies to a seat's centre (m, in the floor plane): a person in the
/// aisle of a 2+2 bus is about 0.46 m from the seat next to the aisle.
pub const SEAT_CLEARANCE_M: f32 = 0.38;
/// Closest to another standing place (m).
pub const PLACE_CLEARANCE_M: f32 = 0.4;
/// Closest to an entry, an exit or a ticket machine (m).
pub const DOOR_CLEARANCE_M: f32 = 0.8;
/// Steepest link (rise over run) that carries standing people; steeper ones are stairs.
pub const MAX_SLOPE: f32 = 0.25;
/// Most added places per unit.
pub const MAX_PER_UNIT: usize = 160;

fn flat(a: Vec3, b: Vec3) -> f32 {
    Vec3::new(a.x - b.x, a.y - b.y, 0.0).length()
}

/// The standing places to add to `cabin` (walkable graph `graph`).
#[must_use]
pub fn standing_room(cabin: &Cabin, graph: &CabinGraph) -> Vec<Place> {
    let keep_free: Vec<Vec3> = cabin
        .entries
        .iter()
        .chain(&cabin.exits)
        .chain(&cabin.ticket_sales)
        .chain(&cabin.stampers)
        .filter_map(|&n| graph.points.get(n).copied())
        .collect();
    let mut added: Vec<Place> = Vec::new();
    let mut per_unit: Vec<usize> = Vec::new();
    for &(a, b) in &graph.links {
        let unit = graph.unit(a);
        if unit != graph.unit(b) {
            continue;
        }
        let (Some(&pa), Some(&pb)) = (graph.points.get(a), graph.points.get(b)) else {
            continue;
        };
        let along = Vec3::new(pb.x - pa.x, pb.y - pa.y, 0.0);
        let length = along.length();
        let rise = (pb.z - pa.z).abs();
        if length < 0.3 || rise > MAX_SLOPE * length {
            continue;
        }
        let dir = along / length;
        let side = Vec3::new(-dir.y, dir.x, 0.0);
        let mut t = SPACING_M * 0.5;
        while t < length {
            let base = pa + (pb - pa) * (t / length);
            for column in [-COLUMN_OFFSET_M, COLUMN_OFFSET_M, 0.0] {
                let position = base + side * column;
                if per_unit.len() <= unit {
                    per_unit.resize(unit + 1, 0);
                }
                if per_unit[unit] >= MAX_PER_UNIT {
                    continue;
                }
                let blocked = keep_free
                    .iter()
                    .any(|d| flat(*d, position) < DOOR_CLEARANCE_M)
                    || cabin.places.iter().filter(|p| p.unit == unit).any(|p| {
                        let min = if p.is_seat() {
                            SEAT_CLEARANCE_M
                        } else {
                            PLACE_CLEARANCE_M
                        };
                        flat(p.position, position) < min
                    })
                    || added
                        .iter()
                        .filter(|p| p.unit == unit)
                        .any(|p| flat(p.position, position) < PLACE_CLEARANCE_M);
                if blocked {
                    continue;
                }
                per_unit[unit] += 1;
                added.push(Place {
                    position,
                    seat_height: 0.0,
                    rotation_deg: 0.0,
                    unit,
                });
            }
            t += SPACING_M;
        }
    }
    added
}
