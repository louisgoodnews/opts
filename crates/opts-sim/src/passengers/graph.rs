//! The path graph of a cabin: shortest ways between pathpoints.

use glam::Vec3;

mod build;
mod query;
mod route;

/// Parts of a unit's paths that are not linked to each other but this close (m) are joined: the
/// file evidently misses a link (the Valiant 7700A's rear unit has no link between its points 9
/// and 10, 0.2 m apart, which cuts its rear half off the doors).
pub const GAP_BRIDGE_M: f32 = 0.5;

/// Walkable graph of a vehicle (links are walkable both ways unless they are one-way).
#[derive(Debug, Clone, Default)]
pub struct CabinGraph {
    /// Position of every pathpoint (segment space: x right, y forward, z up).
    pub points: Vec<Vec3>,
    /// `(from, to)` of every link in file order (the index of `GetHumanCountOnPathLink`).
    pub links: Vec<(usize, usize)>,
    /// Links added across gaps of at most `GAP_BRIDGE_M` (not in [`Self::links`]).
    pub bridges: Vec<(usize, usize)>,
    unit_of: Vec<usize>,
    adjacent: Vec<Vec<(usize, f32)>>,
}
