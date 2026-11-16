//! The O3D mesh path of the content loader.

use super::*;

mod load;
mod part_builder;
mod validate;

/// Parameters for loading an O3D model into model parts.
pub(crate) struct O3dPartsArgs<'a> {
    pub o3d_path: &'a Path,
    pub tex_dirs: &'a [PathBuf],
    pub overrides: &'a [TextureOverride],
    pub keep_skin: bool,
    pub slots: Option<&'a [(String, u16)]>,
    pub bend: Option<(&'a Path, &'a BaseSurface)>,
}
impl ContentLoader<'_> {
    /// Loads and splits an `.o3d` file into one [`ModelPart`] per used material.
    ///
    /// `tex_dirs` are searched for textures before the global `Texture` folder.
    pub fn o3d_parts(
        &mut self,
        o3d_path: &Path,
        tex_dirs: &[PathBuf],
        overrides: &[TextureOverride],
    ) -> Vec<ModelPart> {
        self.o3d_parts_impl(O3dPartsArgs {
            o3d_path,
            tex_dirs,
            overrides,
            keep_skin: false,
            slots: None,
            bend: None,
        })
    }

    /// [`Self::o3d_parts`] bent over a `[crossing_heightdeformation]` base surface: every vertex
    /// is raised by the base height under it. `bend` is `(base file, surface)`; the parts are
    /// cached per base file.
    pub fn o3d_parts_bent(
        &mut self,
        o3d_path: &Path,
        tex_dirs: &[PathBuf],
        overrides: &[TextureOverride],
        bend: Option<(&Path, &BaseSurface)>,
    ) -> Vec<ModelPart> {
        self.o3d_parts_impl(O3dPartsArgs {
            o3d_path,
            tex_dirs,
            overrides,
            keep_skin: false,
            slots: None,
            bend,
        })
    }

    /// The base surface of a `[crossing_heightdeformation]` mesh file (cached; `None` when the
    /// file cannot be read or decoded).
    pub fn base_surface(&mut self, path: &Path) -> Option<std::sync::Arc<BaseSurface>> {
        if let Some(b) = self.cache.bases.get(path) {
            return b.clone();
        }
        let base = match load_decoded_meshes(path) {
            Ok(models) => Some(std::sync::Arc::new(BaseSurface::from_meshes(&models))),
            Err(e) => {
                self.fail(format!("crossing base mesh {}: {e}", path.display()));
                None
            }
        };
        self.cache.bases.insert(path.to_path_buf(), base.clone());
        base
    }

    /// Like [`Self::o3d_parts`], but keeps the bone-weight vertex attributes (for meshes that
    /// are spawned with a `SkinnedMesh`, i.e. humans).
    pub fn o3d_skinned_parts(
        &mut self,
        o3d_path: &Path,
        tex_dirs: &[PathBuf],
        overrides: &[TextureOverride],
    ) -> Vec<ModelPart> {
        self.o3d_parts_impl(O3dPartsArgs {
            o3d_path,
            tex_dirs,
            overrides,
            keep_skin: true,
            slots: None,
            bend: None,
        })
    }

    /// [`Self::o3d_skinned_parts`] for a vehicle mesh whose `[setbone]` entries give the joint
    /// numbers (`(bone name, joint)`); bones the model does not list use the joint after the
    /// highest one.
    pub fn o3d_skinned_parts_slots(
        &mut self,
        o3d_path: &Path,
        tex_dirs: &[PathBuf],
        overrides: &[TextureOverride],
        slots: &[(String, u16)],
    ) -> Vec<ModelPart> {
        self.o3d_parts_impl(O3dPartsArgs {
            o3d_path,
            tex_dirs,
            overrides,
            keep_skin: true,
            slots: Some(slots),
            bend: None,
        })
    }

    fn o3d_parts_impl(&mut self, args: O3dPartsArgs<'_>) -> Vec<ModelPart> {
        let mut scratch = TraceScratch::new();
        let o3d_path = args.o3d_path;
        let parts = self.o3d_parts_core(args, &mut scratch);
        if scratch.lap.active() {
            scratch.finish(self.meshes, o3d_path, &parts);
        }
        parts
    }
}
