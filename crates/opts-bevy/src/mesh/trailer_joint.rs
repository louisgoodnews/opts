//! Trailer-front joint of an articulated bellow: the trailer-end vertices of the bellow move with
//! the trailer's rigid motion, so their weight on the last ring (ring D) goes to a joint of its
//! own. That joint is kept at identity until the articulation drives it.
//!
//! Trailer-end rule (generic, from the articulation geometry, not from the bus name): the axis is
//! the direction from the head's coupling point to the trailer's hitch ([`trailer_end_axis`]). The
//! trailer's body lies on the far side of its hitch, so the trailer end of the bellow is the
//! minimum of the projection on that axis, and the band is the vertices within
//! [`TRAILER_BAND_M`] of that minimum. For the Kajosoft Citybus 530 Facelift bellow the axis is
//! OMSI `+y` (coupling `y = -4.421`, trailer hitch `y = 4.116`). In the o3d file's own axes that
//! is `z` (see [`raw_dir_from_omsi`]): the band holds 75 vertices, all carrying ring D. The
//! lowest-`y` band of the same bellow in the o3d axes is not the trailer face: it is the bottom
//! strip of both ends (252 vertices, of which 60 carry ring D).

use bevy::math::Vec3;

/// One vertex of a skin table: four joint indices and their weights.
pub type SkinEntry = ([u16; 4], [f32; 4]);

/// Thickness of the trailer-end band measured from the trailer-end extreme, in metres.
pub const TRAILER_BAND_M: f32 = 0.03;

/// The unit axis from the head's coupling point toward the trailer's hitch, both given in the same
/// OMSI frame (y forward, z up). `None` when the two points coincide.
#[must_use]
pub fn trailer_end_axis(coupling: Vec3, hitch: Vec3) -> Option<Vec3> {
    let d = hitch - coupling;
    (d.length_squared() > 1e-12).then(|| d.normalize())
}

/// An OMSI direction in the axes of an o3d file (`x`, `z`, `y` of OMSI: the file's height is the
/// OMSI `z`, its length the OMSI `y`).
#[must_use]
pub fn raw_dir_from_omsi(d: Vec3) -> Vec3 {
    Vec3::new(d.x, d.z, d.y)
}

/// An OMSI direction in the Bevy model space of the meshes (`(x, y, z) -> (x, z, -y)`, the
/// vehicle's own conversion of `omsi_to_bevy_pos`).
#[must_use]
pub fn model_dir_from_omsi(d: Vec3) -> Vec3 {
    Vec3::new(d.x, d.z, -d.y)
}

/// The smallest projection of the positions on `axis`, `None` when there are none.
#[must_use]
pub fn band_min(positions: &[Vec3], axis: Vec3) -> Option<f32> {
    let min = positions
        .iter()
        .map(|p| p.dot(axis))
        .fold(f32::INFINITY, f32::min);
    min.is_finite().then_some(min)
}

/// Indices of the vertices whose projection on `axis` lies within [`TRAILER_BAND_M`] of `min`.
#[must_use]
pub fn band_at(positions: &[Vec3], axis: Vec3, min: f32) -> Vec<usize> {
    positions
        .iter()
        .enumerate()
        .filter(|(_, p)| p.dot(axis) <= min + TRAILER_BAND_M)
        .map(|(i, _)| i)
        .collect()
}

/// Indices of the trailer-end vertices of a bellow: the band at the minimum of the projection on
/// `axis` (a unit vector in the positions' frame) over all of them. Empty when there are none.
#[must_use]
pub fn trailer_end_vertices(positions: &[Vec3], axis: Vec3) -> Vec<usize> {
    band_min(positions, axis).map_or_else(Vec::new, |min| band_at(positions, axis, min))
}

/// The joint that carries most of the given entries (positive weight), ties to the lower index.
#[must_use]
pub fn dominant_joint<'a>(entries: impl Iterator<Item = &'a SkinEntry>) -> Option<u16> {
    let mut count: std::collections::BTreeMap<u16, usize> = std::collections::BTreeMap::new();
    for (joints, weights) in entries {
        for (j, w) in joints.iter().zip(weights) {
            if *w > 0.0 {
                *count.entry(*j).or_default() += 1;
            }
        }
    }
    count
        .into_iter()
        .max_by_key(|(j, n)| (*n, std::cmp::Reverse(*j)))
        .map(|(j, _)| j)
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
