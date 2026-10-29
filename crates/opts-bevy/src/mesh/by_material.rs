//! Converts an [`O3d`] into one Bevy [`Mesh`] per material slot.

use std::collections::{BTreeMap, HashMap};

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};

use super::{skin_table_slots, smooth_normal_mesh, O3d, APPLY_O3D_MATRIX};

struct SubmeshBuilder<'a> {
    o3d: &'a O3d,
    xf_pos: Box<dyn Fn(bevy::math::Vec3) -> bevy::math::Vec3 + 'a>,
    xf_dir: Box<dyn Fn(bevy::math::Vec3) -> bevy::math::Vec3 + 'a>,
    mirrored: bool,
    cycled: bool,
    recompute: bool,
    skin: &'a Option<Vec<([u16; 4], [f32; 4])>>,
}

impl<'a> SubmeshBuilder<'a> {
    fn build_submesh(&self, slot: usize, tris: &[[u32; 3]]) -> Option<Mesh> {
        let mut joint_idx: Vec<[u16; 4]> = Vec::new();
        let mut joint_w: Vec<[f32; 4]> = Vec::new();
        let mut remap: HashMap<u32, u32> = HashMap::new();
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut uvs = Vec::new();
        let mut indices = Vec::with_capacity(tris.len() * 3);

        for tri in tris {
            let mut mapped = [0u32; 3];
            let mut ok = true;
            for (k, &vi) in tri.iter().enumerate() {
                let Some(v) = self.o3d.vertices.get(vi as usize) else {
                    ok = false;
                    break;
                };
                mapped[k] = *remap.entry(vi).or_insert_with(|| {
                    let raw = if self.cycled {
                        bevy::math::Vec3::new(-v.position.y, v.position.x, v.position.z)
                    } else {
                        v.position
                    };
                    let p = (self.xf_pos)(raw);
                    let n = (self.xf_dir)(v.normal);
                    positions.push([p.x, p.y, -p.z]);
                    normals.push([n.x, n.y, -n.z]);
                    uvs.push([v.uv.x, v.uv.y]);
                    if let Some(t) = self.skin {
                        let (j, w) = t[vi as usize];
                        joint_idx.push(j);
                        joint_w.push(w);
                    }
                    (positions.len() - 1) as u32
                });
            }
            if !ok {
                tracing::error!(at = %crate::position_tag::position_tag(),
                    slot,
                    "o3d face references a vertex out of range; submesh skipped"
                );
                return None;
            }
            if self.mirrored {
                indices.extend_from_slice(&mapped);
            } else {
                indices.extend_from_slice(&[mapped[0], mapped[2], mapped[1]]);
            }
        }
        if positions.is_empty() {
            tracing::error!(at = %crate::position_tag::position_tag(),
                slot,
                "o3d face references a vertex out of range; submesh skipped"
            );
            return None;
        }

        if !self.recompute {
            align_winding(&positions, &normals, &mut indices);
            let mut mesh = Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::default(),
            );
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
            if self.skin.is_some() {
                mesh.insert_attribute(
                    Mesh::ATTRIBUTE_JOINT_INDEX,
                    bevy::mesh::VertexAttributeValues::Uint16x4(joint_idx),
                );
                mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, joint_w);
            }
            mesh.insert_indices(Indices::U32(indices));
            Some(mesh)
        } else {
            let mut mesh = smooth_normal_mesh(positions, uvs, indices);
            if self.skin.is_some() {
                mesh.insert_attribute(
                    Mesh::ATTRIBUTE_JOINT_INDEX,
                    bevy::mesh::VertexAttributeValues::Uint16x4(joint_idx),
                );
                mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, joint_w);
            }
            Some(mesh)
        }
    }
}

fn align_winding(positions: &[[f32; 3]], normals: &[[f32; 3]], indices: &mut [u32]) {
    let mut agree = 0i64;
    for tri in indices.chunks_exact(3) {
        let p = |i: u32| bevy::math::Vec3::from(positions[i as usize]);
        let n = |i: u32| bevy::math::Vec3::from(normals[i as usize]);
        let face = (p(tri[1]) - p(tri[0])).cross(p(tri[2]) - p(tri[0]));
        let vn = n(tri[0]) + n(tri[1]) + n(tri[2]);
        match face.dot(vn).partial_cmp(&0.0) {
            Some(std::cmp::Ordering::Greater) => agree += 1,
            Some(std::cmp::Ordering::Less) => agree -= 1,
            _ => {}
        }
    }
    if agree < 0 {
        for tri in indices.chunks_exact_mut(3) {
            tri.swap(1, 2);
        }
    }
}

fn collect_material_groups(o3d: &O3d) -> BTreeMap<usize, Vec<[u32; 3]>> {
    let mut groups: BTreeMap<usize, Vec<[u32; 3]>> = BTreeMap::new();
    for (i, tri) in o3d.indices.iter().enumerate() {
        if tri.iter().any(|&vi| {
            o3d.vertices
                .get(vi as usize)
                .is_some_and(|v| !v.position.is_finite() || !v.uv.is_finite())
        }) {
            continue;
        }
        let slot = o3d.face_materials.get(i).copied().unwrap_or(0) as usize;
        groups.entry(slot).or_default().push(*tri);
    }
    groups
}

/// [`o3d_to_bevy_meshes_by_material`](super::o3d_to_bevy_meshes_by_material) with `[setbone]`
/// joint numbers for the skin (see [`skin_table_slots`]).
#[must_use]
pub fn o3d_to_bevy_meshes_by_material_slots(
    o3d: &O3d,
    slots: Option<&[(String, u16)]>,
) -> Vec<(usize, Mesh)> {
    let groups = collect_material_groups(o3d);
    let m = if APPLY_O3D_MATRIX || o3d.version == 0 {
        o3d.transform
    } else {
        None
    }
    .unwrap_or([
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]);

    let xf_pos = Box::new(move |p: bevy::math::Vec3| {
        bevy::math::Vec3::new(
            p.x * m[0][0] + p.y * m[1][0] + p.z * m[2][0] + m[3][0],
            p.x * m[0][1] + p.y * m[1][1] + p.z * m[2][1] + m[3][1],
            p.x * m[0][2] + p.y * m[1][2] + p.z * m[2][2] + m[3][2],
        )
    });
    let xf_dir = Box::new(move |n: bevy::math::Vec3| {
        bevy::math::Vec3::new(
            n.x * m[0][0] + n.y * m[1][0] + n.z * m[2][0],
            n.x * m[0][1] + n.y * m[1][1] + n.z * m[2][1],
            n.x * m[0][2] + n.y * m[1][2] + n.z * m[2][2],
        )
        .normalize_or_zero()
    });
    let det = bevy::math::Mat3::from_cols(
        bevy::math::Vec3::new(m[0][0], m[0][1], m[0][2]),
        bevy::math::Vec3::new(m[1][0], m[1][1], m[1][2]),
        bevy::math::Vec3::new(m[2][0], m[2][1], m[2][2]),
    )
    .determinant();

    let layout = o3d.vertex_layout();
    let cycled = matches!(layout, opts_core::formats::o3d::VertexLayout::AxisCycled);
    let recompute = cycled
        || matches!(
            layout,
            opts_core::formats::o3d::VertexLayout::RecomputedNormals
                | opts_core::formats::o3d::VertexLayout::Unreadable
        );
    let skin = skin_table_slots(o3d, slots);
    let builder = SubmeshBuilder {
        o3d,
        xf_pos,
        xf_dir,
        mirrored: det < 0.0,
        cycled,
        recompute,
        skin: &skin,
    };

    let mut out = Vec::with_capacity(groups.len());
    for (slot, tris) in groups {
        if let Some(mesh) = builder.build_submesh(slot, &tris) {
            out.push((slot, mesh));
        }
    }
    out
}
