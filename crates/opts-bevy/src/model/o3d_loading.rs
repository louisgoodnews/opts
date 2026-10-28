//! The O3D mesh path of the content loader.

use super::*;

mod part_builder;
use part_builder::SlotBuildCtx;

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
        self.o3d_parts_impl(o3d_path, tex_dirs, overrides, false, None, None)
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
        self.o3d_parts_impl(o3d_path, tex_dirs, overrides, false, None, bend)
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
        self.o3d_parts_impl(o3d_path, tex_dirs, overrides, true, None, None)
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
        self.o3d_parts_impl(o3d_path, tex_dirs, overrides, true, Some(slots), None)
    }

    fn o3d_parts_impl(
        &mut self,
        o3d_path: &Path,
        tex_dirs: &[PathBuf],
        overrides: &[TextureOverride],
        keep_skin: bool,
        slots: Option<&[(String, u16)]>,
        bend: Option<(&Path, &BaseSurface)>,
    ) -> Vec<ModelPart> {
        let mut scratch = TraceScratch::new();
        let parts = self.o3d_parts_core(
            o3d_path,
            tex_dirs,
            overrides,
            keep_skin,
            slots,
            bend,
            &mut scratch,
        );
        if scratch.lap.active() {
            scratch.finish(self.meshes, o3d_path, &parts);
        }
        parts
    }

    #[allow(clippy::too_many_arguments)]
    fn o3d_parts_core(
        &mut self,
        o3d_path: &Path,
        tex_dirs: &[PathBuf],
        overrides: &[TextureOverride],
        keep_skin: bool,
        slots: Option<&[(String, u16)]>,
        bend: Option<(&Path, &BaseSurface)>,
        sc: &mut TraceScratch,
    ) -> Vec<ModelPart> {
        let mut repaint: Vec<_> = self.cache.texture_replacements.iter().collect();
        repaint.sort();
        let key = parts_key(
            o3d_path,
            overrides,
            keep_skin,
            &repaint,
            slots,
            bend.map(|b| b.0),
        );
        if let Some(parts) = self.cache.parts.get(&key) {
            sc.stage = "parts-cache";
            if self.cache.protected.contains(o3d_path) {
                self.cache.stats.protected_meshes += 1;
            }
            return parts.clone();
        }

        let _t_miss = opts_logging::stream_trace::span("o3d.parts_miss");
        let t_parse = opts_logging::stream_trace::span("o3d.read_parse");
        let bytes = match std::fs::read(o3d_path) {
            Ok(b) => b,
            Err(e) => {
                sc.stage = "read-failed";
                sc.warn(|| format!("read failed: {e}"));
                self.fail(format!("o3d read failed {}: {e}", o3d_path.display()));
                return Vec::new();
            }
        };
        sc.read_ms = sc.lap.split();
        let mut models: Vec<O3d> = match crate::mesh::parse_mesh_file(o3d_path, &bytes) {
            Ok(m) => m,
            Err(e) => {
                sc.stage = "parse-failed";
                sc.warn(|| format!("parse failed: {e}"));
                self.fail(format!("mesh parse failed {}: {e}", o3d_path.display()));
                return Vec::new();
            }
        };
        sc.parse_ms = sc.lap.split();
        sc.stage = "plain";
        drop(t_parse);
        let is_o3d = o3d_path
            .extension()
            .is_some_and(|e| e.to_string_lossy().eq_ignore_ascii_case("o3d"));
        if is_o3d {
            // Keyed meshes: the exporter's generator is known (`o3d.md`, Addendum 14), so the
            // original positions, texture coordinates and normals come back exactly and the mesh
            // is read like a plain file from here on.
            for m in &mut models {
                if m.encryption_key.is_some() && o3d::exact::decode(m).is_some() {
                    self.cache.stats.exact_meshes += 1;
                    sc.stage = "exact";
                }
            }
            // A keyed file the generator does not cover would still read as `Unreadable`
            // (none is known); it is skipped with a warning rather than drawn scrambled.
            sc.decode_ms = sc.lap.split();
            if models
                .iter()
                .any(|m| matches!(m.vertex_layout(), o3d::VertexLayout::Unreadable))
            {
                sc.stage = "rejected";
                self.cache.stats.protected_meshes += 1;
                self.cache.protected.insert(o3d_path.to_path_buf());
                tracing::warn!(
                    path = %o3d_path.display(),
                    "mesh vertex positions are scrambled and could not be decoded; skipped"
                );
                self.cache.parts.insert(key, Vec::new());
                return Vec::new();
            }
            // A mesh that is hundreds of metres across is not an object: positions are corrupt
            // in a way no plausibility check caught (stray far-away vertices).
            let extent = models
                .iter()
                .map(|m| {
                    let (mut lo, mut hi) = (
                        bevy::math::Vec3::splat(f32::MAX),
                        bevy::math::Vec3::splat(f32::MIN),
                    );
                    for t in &m.indices {
                        for &i in t {
                            if let Some(v) = m.vertices.get(i as usize) {
                                lo = lo.min(v.position);
                                hi = hi.max(v.position);
                            }
                        }
                    }
                    (hi - lo).max_element()
                })
                .fold(0.0_f32, f32::max);
            if extent > 60.0 {
                tracing::info!(path = %o3d_path.display(), extent, layout = ?models.first().map(O3d::vertex_layout), "large mesh");
            }
            let limit = MAX_PLAUSIBLE_EXTENT_M;
            if extent > limit {
                sc.stage = "extent";
                sc.warn(|| format!("extent {extent:.0} m exceeds {limit:.0} m"));
                self.cache.stats.protected_meshes += 1;
                self.cache.protected.insert(o3d_path.to_path_buf());
                tracing::warn!(path = %o3d_path.display(), extent, "mesh extent is implausible; skipped");
                self.cache.parts.insert(key, Vec::new());
                return Vec::new();
            }
            if models.iter().any(|m| {
                matches!(
                    m.vertex_layout(),
                    o3d::VertexLayout::AxisCycled | o3d::VertexLayout::RecomputedNormals
                )
            }) {
                self.cache.stats.axis_cycled_meshes += 1;
                tracing::debug!(path = %o3d_path.display(), "axis-cycled vertex layout; normals recomputed");
            }
        }
        if let Some((_, base)) = bend {
            for m in &mut models {
                base.bend_mesh(m);
            }
        }
        for model in &models {
            self.cache.stats.o3d_files += 1;
            self.cache.stats.o3d_vertices += model.vertices.len();
            tracing::debug!(
                path = %o3d_path.display(),
                vertices = model.vertices.len(),
                triangles = model.indices.len(),
                materials = model.materials.len(),
                "loaded mesh"
            );
        }

        let ctx = SlotBuildCtx {
            o3d_path,
            tex_dirs,
            overrides,
            keep_skin,
            slots,
        };
        let mut parts = Vec::new();
        for model in &models {
            self.build_model_parts(model, &ctx, sc, &mut parts);
        }
        sc.build_ms = sc.lap.split();
        self.cache.parts.insert(key, parts.clone());
        parts
    }
}
