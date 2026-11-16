//! The triangles of a plate mesh: its own axes, the textures' height maps, and the upward faces.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use bevy::prelude::*;

use super::super::road_surf::SurfMap;
use super::super::WorldLoadJob;
use super::LocalTri;

/// The upward triangles of one plate mesh in its own axes, with their textures' height maps.
pub(super) fn local_triangles(
    job: &mut WorldLoadJob,
    mesh: &opts_core::formats::scenery::ScoMesh,
    path: &Path,
    model_dir: &Path,
    bend: Option<&(PathBuf, Arc<opts_core::formats::scenery::BaseSurface>)>,
) -> Vec<LocalTri> {
    let object_dir = model_dir
        .parent()
        .map_or_else(|| model_dir.to_path_buf(), Path::to_path_buf);
    let root = job.install.root().to_path_buf();
    let dirs = [
        object_dir.join("texture"),
        model_dir.to_path_buf(),
        object_dir.clone(),
        job.config.map_dir.join("texture"),
        root.join("Texture"),
    ];
    let Ok(mut models) = opts_bevy::load_decoded_meshes(path) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for m in &mut models {
        if let Some((_, base)) = bend {
            base.bend_mesh(m);
        }
        for (slot, part) in opts_bevy::o3d_to_bevy_meshes_by_material(m) {
            // The texture of the slot: the object's `[matl]` for it, else the mesh file's.
            let texture = mesh
                .materials
                .iter()
                .find(|t| t.material_index == slot)
                .map(|t| t.texture.clone())
                .or_else(|| {
                    m.materials
                        .get(slot)
                        .and_then(|x| x.textures.first().cloned())
                })
                .unwrap_or_default();
            if texture.trim().is_empty() {
                continue;
            }
            let (map, period) = super::super::road_surf_load::surf_of_texture(job, &texture, &dirs);
            out.extend(upward_triangles(&part, &map, period));
        }
    }
    out
}

/// The triangles of `mesh` whose face points up (either winding: flat enough to drive on).
fn upward_triangles(mesh: &Mesh, map: &Arc<SurfMap>, period: Option<f32>) -> Vec<LocalTri> {
    let Some(bevy::mesh::VertexAttributeValues::Float32x3(pos)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        return Vec::new();
    };
    let uvs = match mesh.attribute(Mesh::ATTRIBUTE_UV_0) {
        Some(bevy::mesh::VertexAttributeValues::Float32x2(uv)) => uv.clone(),
        _ => vec![[0.0, 0.0]; pos.len()],
    };
    let index: Vec<u32> = match mesh.indices() {
        Some(i) => i.iter().map(|i| i as u32).collect(),
        None => (0..pos.len() as u32).collect(),
    };
    index
        .chunks_exact(3)
        .filter_map(|t| {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| i as usize);
            let corners = [
                Vec3::from(*pos.get(a)?),
                Vec3::from(*pos.get(b)?),
                Vec3::from(*pos.get(c)?),
            ];
            let normal = (corners[1] - corners[0])
                .cross(corners[2] - corners[0])
                .normalize_or_zero();
            (normal.y.abs() > 0.8).then(|| LocalTri {
                corners,
                uv: [a, b, c].map(|i| Vec2::from(uvs.get(i).copied().unwrap_or([0.0, 0.0]))),
                map: map.clone(),
                period,
            })
        })
        .collect()
}
