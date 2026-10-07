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

use bevy::render::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
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

        if let Some(bevy::render::mesh::VertexAttributeValues::Float32x3(pos)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        {
            all_positions.extend_from_slice(pos);
        }

        if let Some(bevy::render::mesh::VertexAttributeValues::Float32x3(norms)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        {
            all_normals.extend_from_slice(norms);
        }

        if let Some(bevy::render::mesh::VertexAttributeValues::Float32x2(uvs)) =
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
