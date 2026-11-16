//! Close-up views of the text-textured triangles of an object.

use bevy::math::Vec3;

use super::{map_to_bevy, Bounds, Projection, View};

/// Two orthographic close-ups of the text-textured triangles, looking at their front and at
/// their back along the area-weighted normal (the winding decides what is "front", and the
/// harness does not know the loader's convention, so it looks from both sides). Things between
/// the eye and the text beyond 1.5 radii are clipped away, so an occluding part cannot hide
/// the text. `None` without triangles.
#[must_use]
pub fn text_views(tris: &[[Vec3; 3]]) -> Option<[View; 2]> {
    let bounds = Bounds::of(tris.iter().flatten().copied())?;
    // Back-to-back duplicates would cancel: align every normal with the largest triangle's.
    let normals: Vec<Vec3> = tris
        .iter()
        .map(|t| (t[1] - t[0]).cross(t[2] - t[0]))
        .collect();
    let reference = normals
        .iter()
        .copied()
        .max_by(|a, b| a.length().total_cmp(&b.length()))
        .unwrap_or(Vec3::Z);
    let mut n = Vec3::ZERO;
    for v in normals {
        n += if v.dot(reference) < 0.0 { -v } else { v };
    }
    let n = if n.length() > 1e-6 {
        n.normalize()
    } else {
        Vec3::Z
    };
    let c = bounds.centre();
    let r = bounds.radius();
    let dist = r * 1.5 + 0.5;
    let up_map = if n.z.abs() > 0.9 { Vec3::Y } else { Vec3::Z };
    let make = |dir: Vec3| View {
        eye: map_to_bevy(c + dir * dist),
        target: map_to_bevy(c),
        up: map_to_bevy(up_map),
        projection: Projection::Orthographic {
            extent: r * 2.2,
            near: 0.0,
            far: dist + r * 1.5 + 0.5,
        },
    };
    Some([make(n), make(-n)])
}
