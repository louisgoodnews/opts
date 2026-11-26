//! The zone model: what a zone is, where it lies and the messages that announce its changes.
//! The ids, kinds and the plain zone box are in `opts-spatial-zones-core` (ISS-092); the zone
//! component here is the Bevy wrapper around the same data.

use bevy::math::Vec2;
use bevy::prelude::*;

pub use opts_spatial_zones_core::model::{ZoneBox, ZoneId, ZoneKind};
use opts_spatial_zones_core::model::{contains, ZoneShape};

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
        contains(self.min, self.max, p)
    }
}

impl From<ZoneBox> for SpatialZone {
    fn from(z: ZoneBox) -> Self {
        Self {
            id: z.id,
            kind: z.kind,
            min: z.min,
            max: z.max,
        }
    }
}

impl ZoneShape for SpatialZone {
    fn zone_id(&self) -> ZoneId {
        self.id
    }
    fn zone_kind(&self) -> ZoneKind {
        self.kind
    }
    fn zone_min(&self) -> Vec2 {
        self.min
    }
    fn zone_max(&self) -> Vec2 {
        self.max
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
