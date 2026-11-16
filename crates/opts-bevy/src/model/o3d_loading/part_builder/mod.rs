//! Material slot building, alpha composition, and lightmap/nightmap handling for O3D parts.

use super::*;

mod texture_and_material;
use texture_and_material::ResolvedTextures;
mod assemble;
mod materials;
mod trace;

/// Context for building parts from a single decoded [`O3d`] model.
pub(super) struct SlotBuildCtx<'a> {
    pub o3d_path: &'a Path,
    pub tex_dirs: &'a [PathBuf],
    pub overrides: &'a [TextureOverride],
    pub keep_skin: bool,
    pub slots: Option<&'a [(String, u16)]>,
}
impl ContentLoader<'_> {
    pub(super) fn build_model_parts(
        &mut self,
        model: &O3d,
        ctx: &SlotBuildCtx<'_>,
        sc: &mut TraceScratch,
        parts: &mut Vec<ModelPart>,
    ) {
        let t_mesh = opts_logging::stream_trace::span("o3d.meshes");
        let slot_meshes = crate::mesh::o3d_to_bevy_meshes_by_material_slots(model, ctx.slots);
        drop(t_mesh);
        for (slot, mesh) in slot_meshes {
            self.build_slot_part(slot, mesh, model, ctx, sc, parts);
        }
    }
}
