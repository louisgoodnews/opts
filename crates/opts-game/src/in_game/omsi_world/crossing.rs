//! Crossings bent by `[crossing_heightdeformation]` and the ground they lay down
//! (`docs/src/architecture/terrain-carving.md` §4 step 5, `docs/src/formats/text/sco.md`).
//!
//! The base surface of a crossing raises its meshes (through the content loader), its collision
//! boxes, its `[path]`s and its `[splinehelper]` arms; the terrain is then carved under the
//! near-level ground faces of crossings and `[surface]` objects stored at absolute height.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use bevy::prelude::*;
use opts_bevy::{load_decoded_meshes, GroundTriangle};
use opts_core::formats::map::MapObjectInstance;
use opts_core::formats::scenery::{crossing_ground_faces, BaseSurface, SceneryObject};

use super::WorldLoadJob;

/// Ground faces of an object in its mesh axes (x right, y up, z forward).
pub(super) type ObjectFaces = Arc<Vec<[Vec3; 3]>>;

impl WorldLoadJob {
    /// The base surface a `.sco` is bent over: `(base file, surface)`, cached; `None` without
    /// `[crossing_heightdeformation]` or when the base cannot be read.
    pub(super) fn crossing_base(
        &mut self,
        sco_path: &Path,
        sco: &SceneryObject,
    ) -> Option<(PathBuf, Arc<BaseSurface>)> {
        let file = sco.crossing_deformation.as_deref()?;
        let sco_dir = sco_path.parent()?;
        let model_dir = self
            .install
            .find_in(sco_dir, "model")
            .unwrap_or_else(|| sco_dir.join("model"));
        let Some(path) = self.install.find_in(&model_dir, file) else {
            tracing::error!(at = %opts_bevy::position_tag(), sco = %sco_path.display(), base = file, "crossing base mesh not found; object drawn flat");
            return None;
        };
        if let Some(b) = self.crossing_bases.get(&path) {
            return b.clone().map(|b| (path, b));
        }
        let base = match load_decoded_meshes(&path) {
            Ok(m) => Some(Arc::new(BaseSurface::from_meshes(&m))),
            Err(e) => {
                tracing::error!(at = %opts_bevy::position_tag(), base = %path.display(), "crossing base mesh unreadable ({e}); object drawn flat");
                None
            }
        };
        self.crossing_bases.insert(path.clone(), base.clone());
        base.map(|b| (path, b))
    }

    /// The ground faces of an absolute-height crossing or `[surface]` object (bent over its base),
    /// in mesh axes; cached per `.sco`. Empty for other objects.
    fn object_ground_faces(&mut self, rel: &str) -> ObjectFaces {
        let Some((sco_path, sco)) = self.sco(rel) else {
            return Arc::default();
        };
        if let Some(f) = self.ground_faces.get(&sco_path) {
            return f.clone();
        }
        let wanted = sco.has_absolute_height() && (sco.surface || !sco.spline_helpers.is_empty());
        let faces = if wanted && !sco.only_editor {
            let base = self.crossing_base(&sco_path, &sco);
            let sco_dir = sco_path.parent().map(Path::to_path_buf).unwrap_or_default();
            let model_dir = self
                .install
                .find_in(&sco_dir, "model")
                .unwrap_or_else(|| sco_dir.join("model"));
            let mut meshes = Vec::new();
            for m in &sco.meshes {
                let Some(p) = self.install.find_in(&model_dir, &m.file) else {
                    continue;
                };
                let Ok(mut models) = load_decoded_meshes(&p) else {
                    continue;
                };
                if let Some((_, b)) = &base {
                    for o in &mut models {
                        b.bend_mesh(o);
                    }
                }
                meshes.extend(models);
            }
            crossing_ground_faces(&meshes, base.as_ref().map(|(_, b)| b.as_ref()))
        } else {
            Vec::new()
        };
        let faces = Arc::new(faces);
        self.ground_faces.insert(sco_path, faces.clone());
        faces
    }

    /// The `[object]`s of a tile: kept from an earlier read, otherwise read now.
    fn objects_of(&mut self, key: (i32, i32)) -> Option<Arc<Vec<MapObjectInstance>>> {
        if let Some(o) = self.tile_objects.get(&key) {
            return Some(o.clone());
        }
        self.tile_files.get(&key)?;
        let objects = Arc::new(self.read_tile(key)?.objects);
        self.tile_objects.insert(key, objects.clone());
        Some(objects)
    }

    /// Ground faces of the crossings and surface objects of tile `other`, in the frame whose
    /// origin is `other`'s corner moved by `offset` (OMSI axes: east, north, height).
    pub(super) fn crossing_ground_triangles(
        &mut self,
        other: (i32, i32),
        offset: Vec2,
    ) -> Vec<GroundTriangle> {
        let Some(objects) = self.objects_of(other) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for o in objects.iter() {
            let faces = self.object_ground_faces(&o.sco_file);
            if faces.is_empty() {
                continue;
            }
            let (sin, cos) = o.heading_deg.to_radians().sin_cos();
            let place = |p: Vec3| {
                Vec3::new(
                    offset.x + o.position.x + p.x * cos + p.z * sin,
                    offset.y + o.position.y - p.x * sin + p.z * cos,
                    o.position.z + p.y,
                )
            };
            out.extend(faces.iter().map(|f| f.map(place)));
        }
        out
    }
}
