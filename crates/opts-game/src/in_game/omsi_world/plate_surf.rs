//! The unevenness of surface plates (M-0146): junction and street plates (`[surface]` objects)
//! carry their road textures on their visible meshes, not on a spline profile. Their upward
//! triangles keep their texture coordinates and the texture's height map (`.surf`, else its
//! surface type, `road_surf_load::surf_of_texture`), so a wheel on a cobbled junction rattles
//! like one on a cobbled street.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use bevy::prelude::*;

use super::road_surf::SurfMap;
use super::WorldLoadJob;

/// A triangle of a surface plate in the session frame (x east, y north) with its heights, texture
/// coordinates and height map.
#[derive(Debug, Clone)]
pub struct SurfTri {
    /// Corners (m).
    pub xy: [Vec2; 3],
    /// Heights of the corners (m).
    pub z: [f32; 3],
    /// Texture coordinates of the corners.
    pub uv: [Vec2; 3],
    /// The texture's height map.
    pub map: Arc<SurfMap>,
    /// A synthetic map is laid in metres: the length it repeats over (m).
    pub period: Option<f32>,
}

impl SurfTri {
    /// The surface height and the unevenness at `p`, when `p` lies on the triangle.
    #[must_use]
    pub fn at(&self, p: Vec2) -> Option<(f32, f32)> {
        let [a, b, c] = self.xy;
        let d = (b.y - c.y) * (a.x - c.x) + (c.x - b.x) * (a.y - c.y);
        if d.abs() < 1e-9 {
            return None;
        }
        let w0 = ((b.y - c.y) * (p.x - c.x) + (c.x - b.x) * (p.y - c.y)) / d;
        let w1 = ((c.y - a.y) * (p.x - c.x) + (a.x - c.x) * (p.y - c.y)) / d;
        let w2 = 1.0 - w0 - w1;
        let eps = -1e-4;
        if w0 < eps || w1 < eps || w2 < eps {
            return None;
        }
        let z = w0 * self.z[0] + w1 * self.z[1] + w2 * self.z[2];
        let (u, v) = match self.period {
            Some(period) => (p.x / period, p.y / period),
            None => {
                let uv = self.uv[0] * w0 + self.uv[1] * w1 + self.uv[2] * w2;
                (uv.x, uv.y)
            }
        };
        Some((z, self.map.height_at(u, v)))
    }

    /// The triangle's extent (min, max) in x and y.
    #[must_use]
    pub fn bounds(&self) -> (Vec2, Vec2) {
        let lo = self.xy[0].min(self.xy[1]).min(self.xy[2]);
        let hi = self.xy[0].max(self.xy[1]).max(self.xy[2]);
        (lo, hi)
    }
}

/// A triangle of a plate mesh in its own (Bevy) axes, before it is placed.
#[derive(Debug, Clone)]
pub(super) struct LocalTri {
    corners: [Vec3; 3],
    uv: [Vec2; 3],
    map: Arc<SurfMap>,
    period: Option<f32>,
}

/// The cache key of a plate mesh's triangles: the mesh file and the crossing base it is bent over.
pub(super) type PlateKey = (PathBuf, Option<PathBuf>);

/// The upward triangles of a `[surface]` object's visible meshes, placed with `tf` (Bevy world
/// axes), each with its texture's height map. `bend` is the crossing base the object is bent
/// over, as for its collision mesh. A mesh file's triangles are read once (per base).
pub(super) fn plate_triangles(
    job: &mut WorldLoadJob,
    sco: &opts_core::formats::scenery::SceneryObject,
    model_dir: &Path,
    tf: &Transform,
    bend: Option<&(PathBuf, Arc<opts_core::formats::scenery::BaseSurface>)>,
) -> Vec<SurfTri> {
    let mut out = Vec::new();
    for mesh in sco.meshes.iter().filter(|m| !m.only_editor) {
        let Some(path) = job.install.find_in(model_dir, &mesh.file) else {
            continue;
        };
        let key = (path.clone(), bend.map(|b| b.0.clone()));
        let local = match job.plate_tris.get(&key) {
            Some(l) => l.clone(),
            None => {
                let l = Arc::new(local_triangles(job, mesh, &path, model_dir, bend));
                job.plate_tris.insert(key, l.clone());
                l
            }
        };
        out.extend(local.iter().map(|t| {
            let p = t.corners.map(|c| tf.transform_point(c));
            SurfTri {
                // Bevy (x, y up, z back) to the session frame (x east, y north).
                xy: p.map(|c| Vec2::new(c.x, -c.z)),
                z: p.map(|c| c.y),
                uv: t.uv,
                map: t.map.clone(),
                period: t.period,
            }
        }));
    }
    out
}

/// The upward triangles of one plate mesh in its own axes, with their textures' height maps.
fn local_triangles(
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
            let (map, period) = super::road_surf_load::surf_of_texture(job, &texture, &dirs);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_point_on_the_triangle_reads_its_height_and_its_map() {
        let tri = SurfTri {
            xy: [Vec2::ZERO, Vec2::new(4.0, 0.0), Vec2::new(0.0, 4.0)],
            z: [1.0, 1.0, 1.0],
            uv: [Vec2::ZERO, Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0)],
            map: Arc::new(SurfMap::from_heights(1, 1, vec![0.01])),
            period: None,
        };
        let (z, u) = tri.at(Vec2::new(1.0, 1.0)).expect("inside");
        assert!((z - 1.0).abs() < 1e-6 && (u - 0.01).abs() < 1e-6);
        assert!(
            tri.at(Vec2::new(3.0, 3.0)).is_none(),
            "outside the hypotenuse"
        );
        assert_eq!(tri.bounds(), (Vec2::ZERO, Vec2::new(4.0, 4.0)));
    }
}
