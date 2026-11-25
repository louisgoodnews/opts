//! What kind of thing a trace record describes: the `kind` strings of records and probes.

/// A placed scenery object (`.sco`).
pub const SCENERY: &str = "scenery";
/// A tree of the map.
pub const TREE: &str = "tree";
/// An object placed along a spline (attachment).
pub const ATTACHMENT: &str = "attachment";
/// A parked or static vehicle placed as scenery.
pub const PARKED: &str = "parked";
/// A spline segment (road, rail, fence ...).
pub const SPLINE: &str = "spline";
/// The terrain mesh of a tile.
pub const TERRAIN: &str = "terrain";
/// An AI traffic vehicle.
pub const TRAFFIC: &str = "traffic";
/// A pedestrian.
pub const HUMAN: &str = "human";
/// A vehicle of the session (player, session vehicle).
pub const VEHICLE: &str = "vehicle";
/// All kinds, for validation.
pub const ALL: [&str; 9] = [
    SCENERY, TREE, ATTACHMENT, PARKED, SPLINE, TERRAIN, TRAFFIC, HUMAN, VEHICLE,
];
