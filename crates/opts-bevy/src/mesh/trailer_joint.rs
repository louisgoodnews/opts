//! Trailer-front joint of an articulated bellow: the trailer-end vertices of the bellow move with
//! the trailer's rigid motion, so their weight on the last ring (ring D) goes to a joint of its
//! own. That joint is kept at identity until the articulation drives it.
//!
//! Trailer-end rule (geometric, the same for any bellow): the bellow's length axis is given by the
//! caller (the axis along which its rings are stacked). A vertex is trailer-end when its coordinate
//! on that axis lies within [`TRAILER_BAND_M`] of the minimum over all the bellow's vertices, i.e.
//! the face at the trailer end. For the Kajosoft Citybus 530 Facelift bellow the length axis is
//! OMSI `z` (axis 2): the band holds 75 vertices, all carrying ring D. The lowest-`y` band of the
//! same bellow is not the trailer face: it is the bottom strip of both ends (252 vertices, of
//! which 60 carry ring D).

use bevy::math::Vec3;

/// One vertex of a skin table: four joint indices and their weights.
pub type SkinEntry = ([u16; 4], [f32; 4]);

/// Thickness of the trailer-end band measured from the lowest vertex along the length axis, in
/// metres.
pub const TRAILER_BAND_M: f32 = 0.03;

/// Indices of the trailer-end vertices of a bellow, given its vertex positions: the vertices within
/// [`TRAILER_BAND_M`] of the minimum of `axis` (0 = x, 1 = y, 2 = z) over all of them. Empty when no
/// position is finite on that axis.
#[must_use]
pub fn trailer_end_vertices(positions: &[Vec3], axis: usize) -> Vec<usize> {
    assert!(axis < 3, "the length axis is x, y or z");
    let min = positions
        .iter()
        .map(|p| p[axis])
        .fold(f32::INFINITY, f32::min);
    if !min.is_finite() {
        return Vec::new();
    }
    positions
        .iter()
        .enumerate()
        .filter(|(_, p)| p[axis] <= min + TRAILER_BAND_M)
        .map(|(i, _)| i)
        .collect()
}

/// Moves the `ring_d` weight of the given trailer-end vertices to `trailer_joint`. Every other
/// entry, and every other vertex, is copied unchanged. Returns the new table and the number of
/// vertices that had a positive `ring_d` weight and were remapped.
#[must_use]
pub fn remap_trailer_end(
    table: &[SkinEntry],
    trailer_end: &[usize],
    ring_d: u16,
    trailer_joint: u16,
) -> (Vec<SkinEntry>, usize) {
    let mut out = table.to_vec();
    let mut remapped = 0;
    for &vertex in trailer_end {
        let Some((joints, weights)) = out.get_mut(vertex) else {
            continue;
        };
        let mut changed = false;
        for (k, joint) in joints.iter_mut().enumerate() {
            if *joint == ring_d && weights[k] > 0.0 {
                *joint = trailer_joint;
                changed = true;
            }
        }
        remapped += usize::from(changed);
    }
    (out, remapped)
}

/// Rest-pose position of a point under one skin entry: the weighted sum of the joint matrices.
#[cfg(test)]
pub(crate) fn skin_point(entry: &SkinEntry, mats: &[bevy::math::Mat4], p: Vec3) -> Vec3 {
    let (joints, weights) = entry;
    (0..4).fold(Vec3::ZERO, |acc, k| {
        acc + weights[k] * mats[usize::from(joints[k])].transform_point3(p)
    })
}
