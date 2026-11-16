//! One material's submesh, built from its triangles: the vertices are collected, the winding fixed.

use super::attrs::{align_winding, insert_joints};
use super::*;

pub(super) struct SubmeshBuilder<'a> {
    pub(super) o3d: &'a O3d,
    pub(super) xf_pos: Box<dyn Fn(bevy::math::Vec3) -> bevy::math::Vec3 + 'a>,
    pub(super) xf_dir: Box<dyn Fn(bevy::math::Vec3) -> bevy::math::Vec3 + 'a>,
    pub(super) mirrored: bool,
    pub(super) cycled: bool,
    pub(super) recompute: bool,
    pub(super) skin: &'a Option<Vec<([u16; 4], [f32; 4])>>,
}

impl<'a> SubmeshBuilder<'a> {
    /// The mesh of one material from the collected vertices: the file's normals with a winding
    /// check, or smooth normals recomputed; skinning weights are attached when the model has them.
    fn finish_submesh(
        &self,
        positions: Vec<[f32; 3]>,
        normals: Vec<[f32; 3]>,
        uvs: Vec<[f32; 2]>,
        mut indices: Vec<u32>,
        (joint_idx, joint_w): (Vec<[u16; 4]>, Vec<[f32; 4]>),
    ) -> Mesh {
        if self.recompute {
            let mut mesh = smooth_normal_mesh(positions, uvs, indices);
            if self.skin.is_some() {
                insert_joints(&mut mesh, joint_idx, joint_w);
            }
            return mesh;
        }
        align_winding(&positions, &normals, &mut indices);
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        if self.skin.is_some() {
            insert_joints(&mut mesh, joint_idx, joint_w);
        }
        mesh.insert_indices(Indices::U32(indices));
        mesh
    }

    pub(super) fn build_submesh(&self, slot: usize, tris: &[[u32; 3]]) -> Option<Mesh> {
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

        Some(self.finish_submesh(positions, normals, uvs, indices, (joint_idx, joint_w)))
    }
}
