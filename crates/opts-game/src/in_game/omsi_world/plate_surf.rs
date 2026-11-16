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

mod local;
#[cfg(test)]
mod tests;

use local::local_triangles;

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
