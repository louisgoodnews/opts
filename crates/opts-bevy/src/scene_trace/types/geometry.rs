//! Transforms and bounding boxes of trace records, and their union.

use serde::{Deserialize, Serialize};

use super::mesh::MeshNote;

/// A transform in Bevy space.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct TraceTransform {
    /// Translation.
    pub t: [f32; 3],
    /// Rotation as quaternion (x, y, z, w).
    pub q: [f32; 4],
    /// Scale.
    pub s: [f32; 3],
}

/// Local-space bounding box of the thing's geometry.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct TraceBounds {
    /// Smallest corner.
    pub min: [f32; 3],
    /// Largest corner.
    pub max: [f32; 3],
}

/// Union of the bounding boxes of the notes' geometry.
#[must_use]
pub fn union_bounds(notes: &[MeshNote]) -> Option<TraceBounds> {
    let mut out: Option<TraceBounds> = None;
    for b in notes.iter().filter_map(|n| n.bbox) {
        out = Some(match out {
            None => b,
            Some(o) => TraceBounds {
                min: std::array::from_fn(|i| o.min[i].min(b.min[i])),
                max: std::array::from_fn(|i| o.max[i].max(b.max[i])),
            },
        });
    }
    out
}
