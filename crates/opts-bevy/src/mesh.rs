//! Coordinate conversion bridge converting parsed OMSI 3D geometry into Bevy [`Mesh`] assets.
//!
//! OMSI `.o3d` and DirectX `.x` mesh formats use a **left-handed, Y-up** coordinate system
//! with counter-clockwise front-face winding. Bevy uses a **right-handed, Y-up** coordinate
//! system.
//!
//! Per [`coordinates.md`](../../../../docs/src/architecture/coordinates.md):
//! - $X_{\text{bevy}} = X_{\text{omsi}}$
//! - $Y_{\text{bevy}} = Y_{\text{omsi}}$
//! - $Z_{\text{bevy}} = -Z_{\text{omsi}}$
//! - Normal $Z$ components are negated ($N_z \to -N_z$)
//! - Triangle face indices $[i_0, i_1, i_2]$ are reversed to $[i_0, i_2, i_1]$ to preserve
//!   front-face counter-clockwise rasterization after the single-axis reflection.

use bevy::asset::RenderAssetUsages;
use bevy::math::Vec3;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};
use opts_core::formats::o3d::O3d;
use opts_core::formats::x::{XFrame, XMesh, XSubMesh};

/// Converts an in-memory OMSI [`O3d`] mesh into a native Bevy [`Mesh`].
///
/// Converts vertex positions, surface normals, and texture coordinates from OMSI
/// left-handed Y-up space to Bevy right-handed Y-up space by negating the $Z$ component
/// and reversing triangle vertex indices.
#[must_use]
pub fn o3d_to_bevy_mesh(o3d: &O3d) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );

    let mut positions = Vec::with_capacity(o3d.vertices.len());
    let mut normals = Vec::with_capacity(o3d.vertices.len());
    let mut uvs = Vec::with_capacity(o3d.vertices.len());
    let mut colors = Vec::with_capacity(o3d.vertices.len());
    let mut has_colors = false;

    for v in &o3d.vertices {
        // Negate Z for left-handed -> right-handed conversion
        positions.push([v.position.x, v.position.y, -v.position.z]);
        normals.push([v.normal.x, v.normal.y, -v.normal.z]);
        uvs.push([v.uv.x, v.uv.y]);

        if let Some(c) = v.color {
            has_colors = true;
            let normalized = if c[0] > 1.0 || c[1] > 1.0 || c[2] > 1.0 || c[3] > 1.0 {
                [c[0] / 255.0, c[1] / 255.0, c[2] / 255.0, c[3] / 255.0]
            } else {
                c
            };
            colors.push(normalized);
        } else {
            colors.push([1.0, 1.0, 1.0, 1.0]);
        }
    }

    // Invert triangle winding (i0, i1, i2 -> i0, i2, i1) to preserve CCW front-faces
    let mut indices = Vec::with_capacity(o3d.indices.len() * 3);
    for tri in &o3d.indices {
        indices.push(tri[0]);
        indices.push(tri[2]);
        indices.push(tri[1]);
    }

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

    if has_colors {
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    }

    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Per-vertex skinning table `(joint indices, weights)` from the bone-weight chunk, with joint
/// indices following the bone order of the chunk. Each vertex keeps its four strongest
/// influences, normalised to sum to 1; vertices without influence bind fully to joint 0.
#[must_use]
pub fn skin_table(o3d: &O3d) -> Option<Vec<([u16; 4], [f32; 4])>> {
    skin_table_slots(o3d, None)
}

/// [`skin_table`] with the joint numbers of the model's `[setbone]` entries: a bone named there
/// uses the given joint, any other bone of the file (the base bone of a bellows, e.g.
/// `Armature1_Bone`) uses the extra joint after the highest one, which stays on the vehicle
/// (identity). Without this a vertex bound to a bone the model does not list would point
/// past the joint list and be thrown far away.
#[must_use]
pub fn skin_table_slots(
    o3d: &O3d,
    slots: Option<&[(String, u16)]>,
) -> Option<Vec<([u16; 4], [f32; 4])>> {
    let bones = o3d.bone_influences()?;
    let extra = slots.map_or(0, |s| s.iter().map(|b| b.1 + 1).max().unwrap_or(0));
    // Human meshes use the 13 canonical bones; index them in a fixed order so one skeleton
    // fits every character regardless of the order in the file. Other skinned meshes
    // (articulation bellows) keep the order of the chunk.
    let human = bones
        .iter()
        .all(|b| crate::human::HUMAN_BONES.contains(&b.name.as_str()));
    let mut per_vertex: Vec<Vec<(u16, f32)>> = vec![Vec::new(); o3d.vertices.len()];
    for (chunk_idx, bone) in bones.iter().enumerate() {
        let mapped = slots.map(|s| {
            s.iter()
                .find(|(n, _)| n.eq_ignore_ascii_case(&bone.name))
                .map_or(usize::from(extra), |(_, i)| usize::from(*i))
        });
        let bone_idx = if let Some(i) = mapped {
            i
        } else if human {
            crate::human::HUMAN_BONES
                .iter()
                .position(|n| *n == bone.name)
                .unwrap_or(chunk_idx)
        } else {
            chunk_idx
        };
        let Ok(joint) = u16::try_from(bone_idx) else {
            continue;
        };
        for &(vertex, weight) in &bone.entries {
            if weight > 0.0 {
                if let Some(list) = per_vertex.get_mut(vertex as usize) {
                    list.push((joint, weight));
                }
            }
        }
    }
    Some(
        per_vertex
            .into_iter()
            .map(|mut list| {
                list.sort_by(|a, b| b.1.total_cmp(&a.1));
                list.truncate(4);
                let total: f32 = list.iter().map(|e| e.1).sum();
                let mut joints = [0u16; 4];
                let mut weights = [0f32; 4];
                if total > 0.0 {
                    for (i, (j, w)) in list.iter().enumerate() {
                        joints[i] = *j;
                        weights[i] = w / total;
                    }
                } else {
                    weights[0] = 1.0;
                }
                (joints, weights)
            })
            .collect(),
    )
}

/// Whether the `0x79` matrix chunk is applied to vertices (see the comment in
/// [`o3d_to_bevy_meshes_by_material`]); off because vertices are already baked.
pub const APPLY_O3D_MATRIX: bool = false;

/// Converts an [`O3d`] into one Bevy [`Mesh`] per material slot actually referenced by a face.
///
/// Returns `(material_index, mesh)` pairs. Vertices are remapped per submesh and use the same
/// handedness conversion as [`o3d_to_bevy_mesh`].
#[must_use]
pub fn o3d_to_bevy_meshes_by_material(o3d: &O3d) -> Vec<(usize, Mesh)> {
    o3d_to_bevy_meshes_by_material_slots(o3d, None)
}

mod by_material;
pub use by_material::o3d_to_bevy_meshes_by_material_slots;

mod xfile;
pub use xfile::{
    parse_mesh_file, smooth_normal_mesh, x_mesh_to_bevy_meshes, x_mesh_to_combined_bevy_mesh,
    x_submesh_to_bevy_mesh,
};

mod tree;
pub use tree::{tree_billboard_material, tree_billboard_mesh};

#[cfg(test)]
mod tests;
