//! The zone model: what a zone is, where it lies and the messages that announce its changes.

use bevy::math::Vec2;
use bevy::prelude::*;

/// Identifies a zone within one map load.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ZoneId(pub u32);

/// The kinds of zone. Only [`ZoneKind::Tunnel`] exists so far (M-0115 scope).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ZoneKind {
    /// A stretch of road under cover, such as a tunnel.
    Tunnel,
}

/// A zone in the world: an axis-aligned box in the ground plane, built at tile load and dropped at
/// unload. The plane is the one the sampler reads: `x` is east and `y` is the Bevy `z` axis, which
/// is `-north` (ISS-012 corrected this comment; it said `y` north).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct SpatialZone {
    pub id: ZoneId,
    pub kind: ZoneKind,
    /// South-west corner of the box.
    pub min: Vec2,
    /// North-east corner of the box.
    pub max: Vec2,
}

impl SpatialZone {
    /// Whether the ground-plane point `p` lies inside the box (edges included).
    #[must_use]
    pub fn contains(&self, p: Vec2) -> bool {
        p.cmpge(self.min).all() && p.cmple(self.max).all()
    }
}

/// The subject entered the zone. Emitted once per crossing, after the tracker's enter threshold.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoneEntered {
    pub zone: ZoneId,
    pub kind: ZoneKind,
}

/// The subject left the zone. Emitted once per crossing, after the tracker's leave threshold.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoneLeft {
    pub zone: ZoneId,
    pub kind: ZoneKind,
}
