//! The collision bounds of a mesh: the boxes of its triangles, cached per file and bend.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bevy::prelude::*;
use opts_core::formats::scenery::BaseSurface;

use super::super::WorldLoadJob;
use super::mesh_boxes;

/// The index map for `PathBuf` cache keys (kept here so `mod.rs` stays small).
pub(in super::super) type BoundsCache =
    HashMap<(PathBuf, Option<PathBuf>), Option<Vec<(Vec3, Vec3)>>>;
impl WorldLoadJob {
    /// The boxes of a collision mesh in Bevy's local axes, cached (one box for a small mesh, a
    /// set that follows the shape for a large one: see [`mesh_boxes`]); `None` for unreadable
    /// (protected) or missing meshes.
    ///
    /// `bend` is the crossing base the object is bent over (`[crossing_heightdeformation]`).
    pub(super) fn collision_bounds(
        &mut self,
        model_dir: &Path,
        rel: &str,
        bend: Option<&(PathBuf, Arc<BaseSurface>)>,
    ) -> Option<Vec<(Vec3, Vec3)>> {
        let path = self.install.find_in(model_dir, rel)?;
        let key = (path.clone(), bend.map(|b| b.0.clone()));
        if let Some(b) = self.collision_bounds.get(&key) {
            return b.clone();
        }
        let bounds = (|| {
            let mut models = opts_bevy::load_decoded_meshes(&path).ok()?;
            if let Some((_, base)) = bend {
                for m in &mut models {
                    base.bend_mesh(m);
                }
            }
            let (mut positions, mut indices): (Vec<Vec3>, Vec<u32>) = (Vec::new(), Vec::new());
            for m in &models {
                if matches!(
                    m.vertex_layout(),
                    opts_core::formats::o3d::VertexLayout::Unreadable
                ) {
                    return None;
                }
                let mesh = opts_bevy::o3d_to_bevy_mesh(m);
                if let Some(bevy::mesh::VertexAttributeValues::Float32x3(pos)) =
                    mesh.attribute(Mesh::ATTRIBUTE_POSITION)
                {
                    let base = positions.len() as u32;
                    positions.extend(pos.iter().map(|p| Vec3::from(*p)));
                    match mesh.indices() {
                        Some(bevy::mesh::Indices::U32(ix)) => {
                            indices.extend(ix.iter().map(|i| base + i));
                        }
                        Some(bevy::mesh::Indices::U16(ix)) => {
                            indices.extend(ix.iter().map(|i| base + u32::from(*i)));
                        }
                        None => indices.extend(base..base + pos.len() as u32),
                    }
                }
            }
            let boxes = mesh_boxes::boxes_of(&positions, &indices);
            (!boxes.is_empty()).then_some(boxes)
        })();
        self.collision_bounds.insert(key, bounds.clone());
        bounds
    }
}
