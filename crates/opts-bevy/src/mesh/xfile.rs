//! Meshes from DirectX .x files: frames, submeshes, combined meshes and the parser entry.

use super::*;

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
    // One file-wide material palette: the sub-meshes' material lists are concatenated in file
    // order (as a loader that collapses the frame hierarchy into one mesh does) and each
    // sub-mesh's slots are offset into it, so `[matl]` entries (which name a native texture and
    // its occurrence) see the materials of the whole file, as for an `.o3d`. With per-sub-mesh
    // palettes an override for the text quad of a two-frame street sign also matched the plate's
    // slot 0 through the slot-index fallback of `find_override` (Berlin-Spandau report, System
    // 2 item 3). Equal materials of different sub-meshes stay separate slots:
    // `Ruede/Tree_Street_01.sco` gives `[matl_alpha] 1` to occurrence 1 of `Tree_Street_01.tga`,
    // which is the material of the file's third frame (`Plane`), not of the second (`Tube`).
    let mut palette: Vec<&opts_core::formats::x::XMaterial> = Vec::new();
    let mut remaps: Vec<Vec<u16>> = Vec::with_capacity(subs.len());
    for (sub, _) in &subs {
        let local = if sub.materials.is_empty() {
            &x.global_materials
        } else {
            &sub.materials
        };
        let offset = palette.len();
        palette.extend(local.iter());
        remaps.push(
            (0..local.len())
                .map(|i| u16::try_from(offset + i).unwrap_or(u16::MAX))
                .collect(),
        );
    }
    let materials: Vec<opts_core::formats::o3d::Material> = palette
        .iter()
        .map(|m| opts_core::formats::o3d::Material {
            diffuse: m.diffuse,
            specular: m.specular,
            emissive: m.emissive,
            power: m.power,
            textures: m.texture_filename.clone().into_iter().collect(),
        })
        .collect();
    Ok(subs
        .into_iter()
        .zip(remaps)
        .map(|((sub, transform), remap)| O3d {
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
            // A slot outside the sub-mesh's own list stays out of the palette (as before).
            face_materials: sub
                .face_materials
                .iter()
                .map(|&m| {
                    remap.get(m as usize).copied().unwrap_or_else(|| {
                        u16::try_from(materials.len() + m as usize).unwrap_or(u16::MAX)
                    })
                })
                .collect(),
            materials: materials.clone(),
            transform,
            unknown_chunks: Vec::new(),
            encryption_key: None,
            exact_decoded: false,
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
