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
            out.push((slot, smooth_normal_mesh(positions, uvs, indices)));
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

/// Converts a single DirectX [`XSubMesh`] into a native Bevy [`Mesh`].
///
/// Applies the same left-to-right handed conversion, negating $Z$ on positions and normals
/// and reversing index winding order.
#[must_use]
pub fn x_submesh_to_bevy_mesh(submesh: &XSubMesh) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );

    let mut positions = Vec::with_capacity(submesh.vertices.len());
    let mut normals = Vec::with_capacity(submesh.vertices.len());
    let mut uvs = Vec::with_capacity(submesh.vertices.len());

    for v in &submesh.vertices {
        positions.push([v.position.x, v.position.y, -v.position.z]);
        normals.push([v.normal.x, v.normal.y, -v.normal.z]);
        uvs.push([v.uv.x, v.uv.y]);
    }

    let mut indices = Vec::with_capacity(submesh.indices.len() * 3);
    for tri in &submesh.indices {
        indices.push(tri[0]);
        indices.push(tri[2]);
        indices.push(tri[1]);
    }

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Converts all sub-meshes in an [`XMesh`] hierarchy into a flat list of Bevy [`Mesh`] assets.
#[must_use]
pub fn x_mesh_to_bevy_meshes(x_mesh: &XMesh) -> Vec<Mesh> {
    let mut meshes = Vec::new();

    for top_mesh in &x_mesh.top_level_meshes {
        meshes.push(x_submesh_to_bevy_mesh(top_mesh));
    }

    for frame in &x_mesh.frames {
        collect_frame_meshes(frame, &mut meshes);
    }

    meshes
}

fn collect_frame_meshes(frame: &XFrame, meshes: &mut Vec<Mesh>) {
    for mesh in &frame.meshes {
        meshes.push(x_submesh_to_bevy_mesh(mesh));
    }
    for child in &frame.children {
        collect_frame_meshes(child, meshes);
    }
}

/// Converts an [`XMesh`] into a single combined Bevy [`Mesh`] by merging sub-meshes.
///
/// Returns `None` if the input contains zero geometry.
#[must_use]
pub fn x_mesh_to_combined_bevy_mesh(x_mesh: &XMesh) -> Option<Mesh> {
    let submeshes = x_mesh_to_bevy_meshes(x_mesh);
    if submeshes.is_empty() {
        return None;
    }
    if submeshes.len() == 1 {
        return submeshes.into_iter().next();
    }

    // Merge multiple submeshes into a single mesh
    let mut combined = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );

    let mut all_positions = Vec::new();
    let mut all_normals = Vec::new();
    let mut all_uvs = Vec::new();
    let mut all_indices = Vec::new();

    for mesh in submeshes {
        let base_index = all_positions.len() as u32;

        if let Some(bevy::mesh::VertexAttributeValues::Float32x3(pos)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        {
            all_positions.extend_from_slice(pos);
        }

        if let Some(bevy::mesh::VertexAttributeValues::Float32x3(norms)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        {
            all_normals.extend_from_slice(norms);
        }

        if let Some(bevy::mesh::VertexAttributeValues::Float32x2(uvs)) =
            mesh.attribute(Mesh::ATTRIBUTE_UV_0)
        {
            all_uvs.extend_from_slice(uvs);
        }

        if let Some(Indices::U32(idx)) = mesh.indices() {
            for i in idx {
                all_indices.push(base_index + i);
            }
        }
    }

    combined.insert_attribute(Mesh::ATTRIBUTE_POSITION, all_positions);
    combined.insert_attribute(Mesh::ATTRIBUTE_NORMAL, all_normals);
    combined.insert_attribute(Mesh::ATTRIBUTE_UV_0, all_uvs);
    combined.insert_indices(Indices::U32(all_indices));

    Some(combined)
}

pub fn smooth_normal_mesh(positions: Vec<[f32; 3]>, uvs: Vec<[f32; 2]>, indices: Vec<u32>) -> Mesh {
    let mut normals = vec![Vec3::ZERO; positions.len()];
    for tri in indices.chunks_exact(3) {
        let [a, b, c] = [tri[0] as usize, tri[1] as usize, tri[2] as usize];
        let (pa, pb, pc) = (
            Vec3::from(positions[a]),
            Vec3::from(positions[b]),
            Vec3::from(positions[c]),
        );
        let n = (pb - pa).cross(pc - pa);
        normals[a] += n;
        normals[b] += n;
        normals[c] += n;
    }
    let normals: Vec<[f32; 3]> = normals
        .into_iter()
        .map(|n| n.try_normalize().unwrap_or(Vec3::Y).to_array())
        .collect();

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Parses an `.o3d` or DirectX `.x` mesh file into one or more [`O3d`] models.
/// Shared by the asset loaders and the runtime content loader.
///
/// `.x` sub-meshes are normalised into the `.o3d` representation so both formats share the same
/// handedness conversion and material handling.
pub fn parse_mesh_file(path: &std::path::Path, bytes: &[u8]) -> Result<Vec<O3d>, String> {
    let is_x = path
        .extension()
        .is_some_and(|e| e.to_string_lossy().eq_ignore_ascii_case("x"));
    if !is_x {
        return opts_core::formats::o3d::parse(bytes)
            .map(|m| vec![m])
            .map_err(|e| e.to_string());
    }
    let x = opts_core::formats::x::parse_x(bytes).map_err(|e| e.to_string())?;
    let mut subs: Vec<(&opts_core::formats::x::XSubMesh, Option<[[f32; 4]; 4]>)> =
        x.top_level_meshes.iter().map(|m| (m, None)).collect();
    for frame in &x.frames {
        collect_frame_submeshes(frame, None, &mut subs);
    }
    Ok(subs
        .into_iter()
        .map(|(sub, transform)| {
            let palette = if sub.materials.is_empty() {
                &x.global_materials
            } else {
                &sub.materials
            };
            O3d {
                version: 0,
                header_flags: 0,
                vertices: sub
                    .vertices
                    .iter()
                    .map(|v| opts_core::formats::o3d::Vertex {
                        position: v.position,
                        normal: v.normal,
                        uv: v.uv,
                        color: None,
                    })
                    .collect(),
                indices: sub.indices.clone(),
                face_materials: sub.face_materials.iter().map(|&m| m as u16).collect(),
                materials: palette
                    .iter()
                    .map(|m| opts_core::formats::o3d::Material {
                        diffuse: m.diffuse,
                        specular: m.specular,
                        emissive: m.emissive,
                        power: m.power,
                        textures: m.texture_filename.clone().into_iter().collect(),
                    })
                    .collect(),
                transform,
                unknown_chunks: Vec::new(),
                encryption_key: None,
                descrambled: false,
            }
        })
        .collect())
}

/// Row-vector matrix product `a * b` (apply `a` first, then `b`).
fn mul4(a: &[[f32; 4]; 4], b: &[[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut out = [[0.0; 4]; 4];
    for (i, row) in out.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            *cell = (0..4).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    out
}

fn collect_frame_submeshes<'a>(
    frame: &'a opts_core::formats::x::XFrame,
    parent: Option<[[f32; 4]; 4]>,
    out: &mut Vec<(&'a opts_core::formats::x::XSubMesh, Option<[[f32; 4]; 4]>)>,
) {
    let world = match (frame.transform, parent) {
        (Some(local), Some(p)) => Some(mul4(&local, &p)),
        (Some(local), None) => Some(local),
        (None, p) => p,
    };
    for m in &frame.meshes {
        out.push((m, world));
    }
    for child in &frame.children {
        collect_frame_submeshes(child, world, out);
    }
}

/// Builds a crossed-quad billboard for a `[tree]` object: two perpendicular vertical quads,
/// `width` metres wide and `height` metres tall, standing on the origin.
///
/// Normals point up so crowns are lit like foliage rather than flat cards.
#[must_use]
pub fn tree_billboard_mesh(height: f32, width: f32) -> Mesh {
    let hw = width * 0.5;
    let positions = vec![
        [-hw, 0.0, 0.0],
        [hw, 0.0, 0.0],
        [hw, height, 0.0],
        [-hw, height, 0.0],
        [0.0, 0.0, -hw],
        [0.0, 0.0, hw],
        [0.0, height, hw],
        [0.0, height, -hw],
    ];
    let uvs = vec![
        [0.0, 1.0],
        [1.0, 1.0],
        [1.0, 0.0],
        [0.0, 0.0],
        [0.0, 1.0],
        [1.0, 1.0],
        [1.0, 0.0],
        [0.0, 0.0],
    ];
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; 8]);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]));
    mesh
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
}
