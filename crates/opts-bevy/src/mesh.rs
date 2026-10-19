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

/// [`o3d_to_bevy_meshes_by_material`] with `[setbone]` joint numbers for the skin
/// (see [`skin_table_slots`]).
#[must_use]
pub fn o3d_to_bevy_meshes_by_material_slots(
    o3d: &O3d,
    slots: Option<&[(String, u16)]>,
) -> Vec<(usize, Mesh)> {
    use std::collections::BTreeMap;

    let mut groups: BTreeMap<usize, Vec<[u32; 3]>> = BTreeMap::new();
    for (i, tri) in o3d.indices.iter().enumerate() {
        // A non-finite coordinate (observed: `kfznhalterung.o3d` of five vehicles stores `inf`)
        // would make the bounding box of the whole mesh NaN and the entity unculled or invisible;
        // the faces that use such a vertex are skipped (mesh audit P-1014, MI-0005).
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

    // The `0x79` matrix is the exporting object's pivot frame (location/scale/rotation as left
    // in the modelling tool); the vertices are already baked into model space. Applying it
    // displaced or rescaled bodies and wheels (SD77 body 20 m off, GX437 wheels at the origin
    // and doubled in height), while the raw vertices sit exactly at the `[newanim]` pivots of
    // the vehicle config (`vehicle_audit` example, ~8,000 wheel meshes) and, for 16 of 21
    // measurable scenery objects, closer to the declared `[boundingbox]`. See `o3d.md`, Q-006.
    // `.x` frame transforms (version 0 pseudo-files) are real and always applied.
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
    let xf_pos = |p: bevy::math::Vec3| {
        bevy::math::Vec3::new(
            p.x * m[0][0] + p.y * m[1][0] + p.z * m[2][0] + m[3][0],
            p.x * m[0][1] + p.y * m[1][1] + p.z * m[2][1] + m[3][1],
            p.x * m[0][2] + p.y * m[1][2] + p.z * m[2][2] + m[3][2],
        )
    };
    let xf_dir = |n: bevy::math::Vec3| {
        bevy::math::Vec3::new(
            n.x * m[0][0] + n.y * m[1][0] + n.z * m[2][0],
            n.x * m[0][1] + n.y * m[1][1] + n.z * m[2][1],
            n.x * m[0][2] + n.y * m[1][2] + n.z * m[2][2],
        )
        .normalize_or_zero()
    };
    let det = bevy::math::Mat3::from_cols(
        bevy::math::Vec3::new(m[0][0], m[0][1], m[0][2]),
        bevy::math::Vec3::new(m[1][0], m[1][1], m[1][2]),
        bevy::math::Vec3::new(m[2][0], m[2][1], m[2][2]),
    )
    .determinant();
    let mirrored = det < 0.0;

    // Axis-cycled files store (lateral, up, forward) as (-y, x, z) and carry unusable normals.
    // (Footprint test against road centre lines in Berlin-Spandau: with forward = z the buildings
    // clear the roads in 69 of 71 cases, with forward = y in only 63.)
    let layout = o3d.vertex_layout();
    let cycled = matches!(layout, opts_core::formats::o3d::VertexLayout::AxisCycled);
    // Unreadable meshes only get here after the experimental descrambler ran; their stored
    // normals are garbage as well.
    let recompute = cycled
        || matches!(
            layout,
            opts_core::formats::o3d::VertexLayout::RecomputedNormals
                | opts_core::formats::o3d::VertexLayout::Unreadable
        );

    let skin = skin_table_slots(o3d, slots);
    let mut out = Vec::with_capacity(groups.len());
    for (slot, tris) in groups {
        let mut joint_idx: Vec<[u16; 4]> = Vec::new();
        let mut joint_w: Vec<[f32; 4]> = Vec::new();
        let mut remap: std::collections::HashMap<u32, u32> = std::collections::HashMap::new();
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut uvs = Vec::new();
        let mut indices = Vec::with_capacity(tris.len() * 3);
        let mut ok = true;
        for tri in &tris {
            let mut mapped = [0u32; 3];
            for (k, &vi) in tri.iter().enumerate() {
                let Some(v) = o3d.vertices.get(vi as usize) else {
                    ok = false;
                    break;
                };
                mapped[k] = *remap.entry(vi).or_insert_with(|| {
                    let raw = if cycled {
                        bevy::math::Vec3::new(-v.position.y, v.position.x, v.position.z)
                    } else {
                        v.position
                    };
                    let p = xf_pos(raw);
                    let n = xf_dir(v.normal);
                    positions.push([p.x, p.y, -p.z]);
                    normals.push([n.x, n.y, -n.z]);
                    uvs.push([v.uv.x, v.uv.y]);
                    if let Some(t) = &skin {
                        let (j, w) = t[vi as usize];
                        joint_idx.push(j);
                        joint_w.push(w);
                    }
                    (positions.len() - 1) as u32
                });
            }
            if !ok {
                break;
            }
            if mirrored {
                indices.extend_from_slice(&mapped);
            } else {
                indices.extend_from_slice(&[mapped[0], mapped[2], mapped[1]]);
            }
        }
        if !ok || positions.is_empty() {
            tracing::error!(at = %crate::position_tag::position_tag(),
                slot,
                "o3d face references a vertex out of range; submesh skipped"
            );
            continue;
        }
        if !recompute {
            // Make the winding agree with the stored normals: if most faces would point against
            // their own vertex normals, the mesh was authored with the opposite winding and
            // back-face culling would remove its outer surface (a bus with no roof).
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
        if recompute {
            let mut mesh = smooth_normal_mesh(positions, uvs, indices);
            // The vertex order is kept, so a skinned character with cycled axes keeps its weights.
            if skin.is_some() {
                mesh.insert_attribute(
                    Mesh::ATTRIBUTE_JOINT_INDEX,
                    bevy::mesh::VertexAttributeValues::Uint16x4(joint_idx),
                );
                mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, joint_w);
            }
            out.push((slot, mesh));
            continue;
        }
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        if skin.is_some() {
            mesh.insert_attribute(
                Mesh::ATTRIBUTE_JOINT_INDEX,
                bevy::mesh::VertexAttributeValues::Uint16x4(joint_idx),
            );
            mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, joint_w);
        }
        mesh.insert_indices(Indices::U32(indices));
        out.push((slot, mesh));
    }
    out
}

#[cfg(test)]
mod finite_tests {
    use super::*;
    use bevy::math::{Vec2, Vec3};
    use opts_core::formats::o3d::Vertex;

    fn vertex(p: Vec3) -> Vertex {
        Vertex {
            position: p,
            normal: Vec3::Y,
            uv: Vec2::ZERO,
            color: None,
        }
    }

    #[test]
    fn faces_with_non_finite_vertices_are_skipped() {
        let o = O3d {
            version: 1,
            header_flags: 0,
            vertices: vec![
                vertex(Vec3::ZERO),
                vertex(Vec3::X),
                vertex(Vec3::Z),
                vertex(Vec3::new(f32::INFINITY, 0.0, 0.0)),
            ],
            indices: vec![[0, 1, 2], [0, 1, 3]],
            face_materials: vec![0, 0],
            materials: Vec::new(),
            transform: None,
            encryption_key: None,
            exact_decoded: false,
            unknown_chunks: Vec::new(),
        };
        let meshes = o3d_to_bevy_meshes_by_material(&o);
        assert_eq!(meshes.len(), 1);
        let n = meshes[0].1.indices().map_or(0, bevy::mesh::Indices::len);
        assert_eq!(n, 3, "only the finite triangle remains");
    }
}

mod xfile;
pub use xfile::{
    parse_mesh_file, smooth_normal_mesh, x_mesh_to_bevy_meshes, x_mesh_to_combined_bevy_mesh,
    x_submesh_to_bevy_mesh,
};

mod tree;
pub use tree::{tree_billboard_material, tree_billboard_mesh};

#[cfg(test)]
mod x_palette_tests {
    use super::*;

    /// Two frames like `strschild_1.x`: a plate with `StrSchild_1.bmp` and a text quad with
    /// `StrSchild_Text1.bmp`, each with its own one-entry material list.
    pub(crate) const TWO_FRAME_SIGN: &[u8] = b"xof 0303txt 0032\n\
Frame Plate {\n\
  Mesh {\n\
    3; 0.0;0.0;0.0;, 1.0;0.0;0.0;, 0.0;1.0;0.0;;\n\
    1; 3; 0,1,2;;\n\
    MeshMaterialList { 1; 1; 0;\n\
      Material Mat1 { 1.0;1.0;1.0;1.0;; 0.0; 0.0;0.0;0.0;; 0.0;0.0;0.0;;\n\
        TextureFilename { \"StrSchild_1.bmp\"; } }\n\
    }\n\
  }\n\
}\n\
Frame Text {\n\
  Mesh {\n\
    3; 0.0;0.0;0.1;, 1.0;0.0;0.1;, 0.0;1.0;0.1;;\n\
    1; 3; 0,1,2;;\n\
    MeshMaterialList { 1; 1; 0;\n\
      Material Mat1 { 1.0;1.0;1.0;1.0;; 0.0; 0.0;0.0;0.0;; 0.0;0.0;0.0;;\n\
        TextureFilename { \"StrSchild_Text1.bmp\"; } }\n\
    }\n\
  }\n\
}\n";

    #[test]
    fn x_sub_meshes_share_the_file_wide_palette() {
        let models =
            parse_mesh_file(std::path::Path::new("strschild_1.x"), TWO_FRAME_SIGN).expect("parse");
        assert_eq!(models.len(), 2);
        for m in &models {
            let names: Vec<_> = m.materials.iter().map(|m| m.primary_texture()).collect();
            assert_eq!(
                names,
                vec![Some("StrSchild_1.bmp"), Some("StrSchild_Text1.bmp")]
            );
        }
        assert_eq!(models[0].face_materials, vec![0]);
        assert_eq!(models[1].face_materials, vec![1]);
    }

    #[test]
    fn text_override_does_not_reach_the_plate() {
        let models =
            parse_mesh_file(std::path::Path::new("strschild_1.x"), TWO_FRAME_SIGN).expect("parse");
        let over = [crate::model::TextureOverride {
            slot: 0,
            texture: "StrSchild_Text1.bmp".into(),
            alpha: Some(2),
            alpha_variable: None,
            trans_map: None,
            image: None,
            light_map: None,
        }];
        let palette = &models[0].materials;
        assert!(crate::model::find_override(palette, 0, &over).is_none());
        assert!(crate::model::find_override(palette, 1, &over).is_some());
    }

    #[test]
    fn equal_materials_of_two_sub_meshes_stay_two_slots() {
        let doubled = std::str::from_utf8(TWO_FRAME_SIGN)
            .expect("utf8")
            .replace("StrSchild_Text1.bmp", "StrSchild_1.bmp");
        let models =
            parse_mesh_file(std::path::Path::new("a.x"), doubled.as_bytes()).expect("parse");
        assert_eq!(models[0].materials.len(), 2);
        assert_eq!(models[1].face_materials, vec![1]);
        // `[matl] StrSchild_1.bmp 1` reaches the second sub-mesh only (occurrence 1).
        let over = [crate::model::TextureOverride {
            slot: 1,
            texture: "StrSchild_1.bmp".into(),
            alpha: Some(1),
            alpha_variable: None,
            trans_map: None,
            image: None,
            light_map: None,
        }];
        let palette = &models[0].materials;
        assert!(crate::model::find_override(palette, 0, &over).is_none());
        assert!(crate::model::find_override(palette, 1, &over).is_some());
    }
}

#[cfg(test)]
mod tree_tests {
    use super::*;

    #[test]
    fn billboard_has_two_crossed_quads() {
        let m = tree_billboard_mesh(16.0, 18.0);
        assert_eq!(m.count_vertices(), 8);
        assert_eq!(m.indices().map(bevy::mesh::Indices::len), Some(12));
    }

    #[test]
    fn billboard_normals_are_crown_normals() {
        let m = tree_billboard_mesh(16.0, 18.0);
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(pos)) =
            m.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("positions");
        };
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(nrm)) =
            m.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("normals");
        };
        for (p, n) in pos.iter().zip(nrm) {
            let n = Vec3::from(*n);
            assert!((n.length() - 1.0).abs() < 1e-5);
            assert!((n.y - 0.8).abs() < 1e-5, "up component {n}");
            let outward = Vec3::new(p[0], 0.0, p[2]).normalize();
            assert!((Vec3::new(n.x, 0.0, n.z).dot(outward) - 0.6).abs() < 1e-5);
        }
    }

    #[test]
    fn billboard_material_draws_both_faces_without_normal_flip() {
        let m = tree_billboard_material(bevy::asset::Handle::default());
        assert!(!m.double_sided);
        assert_eq!(m.cull_mode, None);
        assert!(m.reflectance.abs() < f32::EPSILON);
        assert!(matches!(m.alpha_mode, bevy::prelude::AlphaMode::Mask(_)));
    }
}
