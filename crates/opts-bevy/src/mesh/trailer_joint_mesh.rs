//! The trailer-end remap on the meshes of a bellow: the skin attributes of its parts are read,
//! remapped as one bellow (one band and one ring for all parts) and written back.

use bevy::asset::{Assets, Handle};
use bevy::math::Vec3;
use bevy::mesh::VertexAttributeValues;
use bevy::prelude::Mesh;

use super::trailer_joint::{band_at, band_min, dominant_joint, remap_trailer_end, SkinEntry};

/// Positions and skin entries of a skinned mesh, `None` when it is not skinned.
fn skin_of(mesh: &Mesh) -> Option<(Vec<Vec3>, Vec<SkinEntry>)> {
    let VertexAttributeValues::Float32x3(pos) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)? else {
        return None;
    };
    let VertexAttributeValues::Uint16x4(idx) = mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX)? else {
        return None;
    };
    let VertexAttributeValues::Float32x4(weights) = mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT)?
    else {
        return None;
    };
    if idx.len() != pos.len() || weights.len() != pos.len() {
        return None;
    }
    let positions = pos.iter().map(|p| Vec3::from(*p)).collect();
    Some((
        positions,
        idx.iter().copied().zip(weights.iter().copied()).collect(),
    ))
}

/// Writes the skin entries back into the joint attributes of a mesh.
fn write_skin(mesh: &mut Mesh, table: &[SkinEntry]) {
    let joints: Vec<[u16; 4]> = table.iter().map(|e| e.0).collect();
    let weights: Vec<[f32; 4]> = table.iter().map(|e| e.1).collect();
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_JOINT_INDEX,
        VertexAttributeValues::Uint16x4(joints),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, weights);
}

/// Remaps the trailer end of one bellow whose skinned parts are `handles`, with `axis` the
/// trailer-end axis in the Bevy model space of the meshes (see
/// [`super::trailer_joint::model_dir_from_omsi`]). The band is the trailer-end face of all parts
/// together, its ring the joint that carries most of it, and that ring's weight there moves to
/// `trailer_joint`. `(bones, trailer_joint)`: the ring must be one of the `bones` joints, else
/// nothing changes. Returns the ring, `None` when no part has a trailer end on a bone.
#[must_use]
pub fn remap_bellow_trailer_end(
    meshes: &mut Assets<Mesh>,
    handles: &[Handle<Mesh>],
    axis: Vec3,
    (bones, trailer_joint): (u16, u16),
) -> Option<u16> {
    let parts: Vec<(Handle<Mesh>, Vec<Vec3>, Vec<SkinEntry>)> = handles
        .iter()
        .filter_map(|h| {
            let (p, t) = skin_of(meshes.get(h)?)?;
            Some((h.clone(), p, t))
        })
        .collect();
    let min = parts
        .iter()
        .filter_map(|(_, p, _)| band_min(p, axis))
        .fold(f32::INFINITY, f32::min);
    if !min.is_finite() {
        return None;
    }
    let bands: Vec<Vec<usize>> = parts
        .iter()
        .map(|(_, p, _)| band_at(p, axis, min))
        .collect();
    let ring = dominant_joint(
        parts
            .iter()
            .zip(&bands)
            .flat_map(|((_, _, t), b)| b.iter().map(move |&i| &t[i])),
    )?;
    if ring >= bones {
        return None;
    }
    for ((handle, _, table), band) in parts.iter().zip(&bands) {
        let (out, n) = remap_trailer_end(table, band, ring, trailer_joint);
        if n > 0 {
            if let Some(mut mesh) = meshes.get_mut(handle) {
                write_skin(&mut mesh, &out);
            }
        }
    }
    Some(ring)
}
