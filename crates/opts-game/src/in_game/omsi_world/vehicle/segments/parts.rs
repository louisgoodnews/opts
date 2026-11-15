//! The o3d parts of one mesh: their overrides, their loading and the body's bounds.

use super::overrides::{build_overrides, in_file_order, mesh_stem};
use super::state::{LoadedMesh, ModelMeshDef, SegmentAssets, SegmentState};
use super::*;

impl<'a> SegmentState<'a> {
    pub(super) fn load_parts(
        &mut self,
        job: &mut WorldLoadJob,
        commands: &mut Commands,
        assets: &mut SegmentAssets,
        mesh_def: &ModelMeshDef,
    ) -> Option<LoadedMesh> {
        let Some(o3d_path) = job.install.find_in(self.model_dir, &mesh_def.path) else {
            tracing::error!(at = %opts_bevy::position_tag(), mesh = %mesh_def.path, "vehicle o3d not found");
            job.cache
                .stats
                .failures
                .push(format!("vehicle o3d not found: {}", mesh_def.path));
            return None;
        };
        let overrides = build_overrides(
            (job, commands, assets.images),
            (mesh_def, self.owner, self.root),
            self.tex_dirs,
            (&mut self.text_images, &mut self.script_images),
        );
        let mut loader = ContentLoader {
            install: &job.install,
            cache: &mut job.cache,
            meshes: assets.meshes,
            materials: assets.materials,
            images: assets.images,
        };
        super::super::super::trace::mesh_used(&o3d_path);
        let skinned = mesh_def.smoothskin && !mesh_def.bones.is_empty();
        let parts = if skinned {
            let slots: Vec<(String, u16)> = mesh_def
                .bones
                .iter()
                .filter_map(|b| u16::try_from(b.index).ok().map(|i| (b.name.clone(), i)))
                .collect();
            loader.o3d_skinned_parts_slots(&o3d_path, self.tex_dirs, &overrides, &slots)
        } else {
            loader.o3d_parts(&o3d_path, self.tex_dirs, &overrides)
        };
        let copy = *self
            .repeats
            .entry(mesh_stem(&mesh_def.path))
            .and_modify(|n| *n += 1)
            .or_insert(0);
        let parts = in_file_order(job, assets.materials, parts, copy);
        Some(LoadedMesh { parts, skinned })
    }

    /// Counts the parts and grows the bounding box by the ones that belong to the body.
    pub(super) fn grow_bounds(&mut self, meshes: &Assets<Mesh>, parts: &[opts_bevy::ModelPart]) {
        for part in parts {
            if let Some(mesh) = meshes.get(&part.mesh) {
                if let Some(bevy::mesh::VertexAttributeValues::Float32x3(pos)) =
                    mesh.attribute(Mesh::ATTRIBUTE_POSITION)
                {
                    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
                    for p in pos {
                        lo = lo.min(Vec3::from(*p));
                        hi = hi.max(Vec3::from(*p));
                    }
                    // Camera framing uses the vehicle body only: meshes that are huge or far
                    // from the origin (shadow volumes, sky/helper geometry) would skew it.
                    if (hi - lo).max_element() < 16.0 && ((hi + lo) * 0.5).abs().max_element() < 8.0
                    {
                        self.min = self.min.min(lo);
                        self.max = self.max.max(hi);
                    }
                }
            }
            self.vertex_count += part.vertices;
            self.mesh_count += 1;
        }
    }
}
