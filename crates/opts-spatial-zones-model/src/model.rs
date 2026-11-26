//! The pure zone model: ids, kinds, the zone box as plain data and the shape the sampler and the
//! query read. The Bevy component wraps [`ZoneBox`] in `opts-spatial-zones` (ISS-092).

use glam::Vec2;

/// Identifies a zone within one map load.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ZoneId(pub u32);

/// The kinds of zone. Only [`ZoneKind::Tunnel`] exists so far (M-0115 scope).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ZoneKind {
    /// A stretch of road under cover, such as a tunnel.
    Tunnel,
}

/// A zone as plain data: an axis-aligned box in the ground plane. The plane is the one the sampler
/// reads: `x` is east and `y` is the Bevy `z` axis, which is `-north`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoneBox {
    pub id: ZoneId,
    pub kind: ZoneKind,
    /// South-west corner of the box.
    pub min: Vec2,
    /// North-east corner of the box.
    pub max: Vec2,
}

/// Whether the ground-plane point `p` lies inside the box `min`..`max` (edges included).
#[must_use]
pub fn contains(min: Vec2, max: Vec2, p: Vec2) -> bool {
    p.cmpge(min).all() && p.cmple(max).all()
}

/// A zone the sampler and the query can read: its id, kind and box.
pub trait ZoneShape {
    fn zone_id(&self) -> ZoneId;
    fn zone_kind(&self) -> ZoneKind;
    fn zone_min(&self) -> Vec2;
    fn zone_max(&self) -> Vec2;

    /// Whether `p` lies inside the zone's box (edges included).
    fn contains_point(&self, p: Vec2) -> bool {
        contains(self.zone_min(), self.zone_max(), p)
    }
}

impl ZoneShape for ZoneBox {
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
