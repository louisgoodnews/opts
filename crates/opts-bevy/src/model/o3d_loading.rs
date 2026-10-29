//! The O3D mesh path of the content loader.

use super::*;

mod part_builder;
use part_builder::SlotBuildCtx;

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

    fn o3d_parts_core(&mut self, args: O3dPartsArgs<'_>, sc: &mut TraceScratch) -> Vec<ModelPart> {
        let mut repaint: Vec<_> = self.cache.texture_replacements.iter().collect();
        repaint.sort();
        let key = parts_key(
            args.o3d_path,
            args.overrides,
            args.keep_skin,
            &repaint,
            args.slots,
            args.bend.map(|b| b.0),
        );
        if let Some(parts) = self.cache.parts.get(&key) {
            sc.stage = "parts-cache";
            if self.cache.protected.contains(args.o3d_path) {
                self.cache.stats.protected_meshes += 1;
            }
            return parts.clone();
        }

        let _t_miss = opts_logging::stream_trace::span("o3d.parts_miss");
        let Some(mut models) = self.read_and_parse_mesh(args.o3d_path, sc) else {
            return Vec::new();
        };

        if !decode_and_validate_o3d(&mut models, args.o3d_path, sc, self.cache) {
            self.cache.parts.insert(key, Vec::new());
            return Vec::new();
        }
        if let Some((_, base)) = args.bend {
            for m in &mut models {
                base.bend_mesh(m);
            }
        }
        for model in &models {
            self.cache.stats.o3d_files += 1;
            self.cache.stats.o3d_vertices += model.vertices.len();
            tracing::debug!(
                path = %args.o3d_path.display(),
                vertices = model.vertices.len(),
                triangles = model.indices.len(),
                materials = model.materials.len(),
                "loaded mesh"
            );
        }

        let ctx = SlotBuildCtx {
            o3d_path: args.o3d_path,
            tex_dirs: args.tex_dirs,
            overrides: args.overrides,
            keep_skin: args.keep_skin,
            slots: args.slots,
        };
        let mut parts = Vec::new();
        for model in &models {
            self.build_model_parts(model, &ctx, sc, &mut parts);
        }
        sc.build_ms = sc.lap.split();
        self.cache.parts.insert(key, parts.clone());
        parts
    }

    fn read_and_parse_mesh(&mut self, path: &Path, sc: &mut TraceScratch) -> Option<Vec<O3d>> {
        let t_parse = opts_logging::stream_trace::span("o3d.read_parse");
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                sc.stage = "read-failed";
                sc.warn(|| format!("read failed: {e}"));
                self.fail(format!("o3d read failed {}: {e}", path.display()));
                return None;
            }
        };
        sc.read_ms = sc.lap.split();
        let models: Vec<O3d> = match crate::mesh::parse_mesh_file(path, &bytes) {
            Ok(m) => m,
            Err(e) => {
                sc.stage = "parse-failed";
                sc.warn(|| format!("parse failed: {e}"));
                self.fail(format!("mesh parse failed {}: {e}", path.display()));
                return None;
            }
        };
        sc.parse_ms = sc.lap.split();
        sc.stage = "plain";
        drop(t_parse);
        Some(models)
    }
}

fn decode_and_validate_o3d(
    models: &mut [O3d],
    o3d_path: &Path,
    sc: &mut TraceScratch,
    cache: &mut ContentCache,
) -> bool {
    let is_o3d = o3d_path
        .extension()
        .is_some_and(|e| e.to_string_lossy().eq_ignore_ascii_case("o3d"));
    if !is_o3d {
        return true;
    }
    for m in models.iter_mut() {
        if m.encryption_key.is_some() && o3d::exact::decode(m).is_some() {
            cache.stats.exact_meshes += 1;
            sc.stage = "exact";
        }
    }
    sc.decode_ms = sc.lap.split();
    if models
        .iter()
        .any(|m| matches!(m.vertex_layout(), o3d::VertexLayout::Unreadable))
    {
        sc.stage = "rejected";
        cache.stats.protected_meshes += 1;
        cache.protected.insert(o3d_path.to_path_buf());
        tracing::warn!(path = %o3d_path.display(), "mesh vertex positions are scrambled and could not be decoded; skipped");
        return false;
    }
    let extent = models.iter().map(model_extent).fold(0.0_f32, f32::max);
    if extent > 60.0 {
        tracing::info!(path = %o3d_path.display(), extent, layout = ?models.first().map(O3d::vertex_layout), "large mesh");
    }
    if extent > MAX_PLAUSIBLE_EXTENT_M {
        sc.stage = "extent";
        sc.warn(|| format!("extent {extent:.0} m exceeds {MAX_PLAUSIBLE_EXTENT_M:.0} m"));
        cache.stats.protected_meshes += 1;
        cache.protected.insert(o3d_path.to_path_buf());
        tracing::warn!(path = %o3d_path.display(), extent, "mesh extent is implausible; skipped");
        return false;
    }
    if models.iter().any(|m| {
        matches!(
            m.vertex_layout(),
            o3d::VertexLayout::AxisCycled | o3d::VertexLayout::RecomputedNormals
        )
    }) {
        cache.stats.axis_cycled_meshes += 1;
        tracing::debug!(path = %o3d_path.display(), "axis-cycled vertex layout; normals recomputed");
    }
    true
}

fn model_extent(m: &O3d) -> f32 {
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
}
