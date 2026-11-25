//! The geometry of a traffic unit: its model's level of detail and its length.

use super::super::*;

/// The meshes of one unit: the plain `[LOD] 0` body for cars; for buses the full-detail group
/// seen from outside, with the AI-only meshes (`[viewpoint]` 4: interiors of the AI model) and
/// without the cockpit-only ones.
pub(in super::super) fn unit_lod(unit: &Unit) -> Option<ModelLod> {
    if unit.profile.has_cabin {
        unit.model.detail_lod().map(|mut lod| {
            lod.meshes.retain(|m| {
                let raw = m.viewpoint.raw();
                raw == 0 || raw & 1 != 0 || raw & 4 != 0
            });
            for m in &mut lod.meshes {
                if m.viewpoint.raw() & 4 != 0 {
                    m.viewpoint = opts_core::formats::vehicles::model::ViewpointFlags::ALWAYS;
                }
            }
            lod
        })
    } else {
        unit.model
            .lods
            .iter()
            .find(|l| l.threshold == 0.0)
            .or_else(|| unit.model.lods.first())
            .cloned()
    }
}

/// The shortest road vehicle measured as such (m): a bicycle is 1.76 m long.
const MIN_LENGTH_M: f32 = 0.5;

/// A unit's length and the distance of its origin behind the front (m), from its meshes' extent
/// along the vehicle (`min_z … max_z`). Bicycles and mopeds are under 2 m (`[boundingbox]` 1.76 …
/// 1.83 m); a mesh that cannot be measured takes its `[boundingbox]` length (`boxed`), else a
/// car's.
pub(in super::super) fn unit_size(min_z: f32, max_z: f32, boxed: Option<f32>) -> (f32, f32) {
    let len = max_z - min_z;
    if min_z.is_finite() && (MIN_LENGTH_M..25.0).contains(&len) {
        (len, (-min_z).clamp(0.0, len + 2.0))
    } else if let Some(len) = boxed.filter(|l| (MIN_LENGTH_M..25.0).contains(l)) {
        (len, len * 0.5)
    } else {
        (4.4, 2.2)
    }
}
