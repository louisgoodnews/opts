//! Runtime loading of OMSI models and textures from an installation directory.
//!
//! [`ContentLoader`] turns a mesh file plus its material/texture overrides (from a `.sco` or a
//! vehicle `model.cfg`) into [`ModelPart`]s: one mesh per material slot, with textures resolved
//! through [`OmsiInstall`]. It reuses the crate's mesh, material and texture converters and
//! is shared by scenery objects, splines and vehicles.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use bevy::asset::{Assets, Handle};
use bevy::ecs::entity::Entity;
use bevy::ecs::name::Name;
use bevy::ecs::system::Commands;
use bevy::image::{Image, ImageAddressMode, ImageSampler, ImageSamplerDescriptor};
use bevy::math::Vec3;
use bevy::mesh::Mesh;
use bevy::mesh::Mesh3d;
use bevy::pbr::MeshMaterial3d;
use bevy::pbr::StandardMaterial;
use bevy::prelude::AlphaMode;

use opts_core::formats::o3d::{self, O3d};
use opts_core::formats::vehicles::model::LightMap;

use crate::install::OmsiInstall;
use crate::material::o3d_material_to_standard_material;
use crate::texture::load_bevy_image;

/// Scrambled (keyed) meshes larger than this along any axis are rejected as corrupt (metres).
pub const MAX_PLAUSIBLE_EXTENT_M: f32 = 150.0;

/// Plain meshes are trusted far more: road and crossing models are legitimately hundreds of
/// metres long (up to ~700 m observed), so only absurd sizes count as corrupt.
pub const MAX_PLAUSIBLE_PLAIN_EXTENT_M: f32 = 3000.0;

/// Texture override for one `.o3d` material slot (from `[matl]` in `.sco` or `model.cfg`).
#[derive(Debug, Clone, PartialEq)]
pub struct TextureOverride {
    /// `.o3d` material index the texture applies to.
    pub slot: usize,
    /// Texture file reference.
    pub texture: String,
    /// `[matl_alpha]` mode: `1` = cutout (alpha test), `2` = blend; `None` = opaque.
    pub alpha: Option<u32>,
    /// The material's alpha is scaled by a script variable (`[alphascale]`), whose default of 0
    /// makes it invisible (dirt, rain and condensation overlays).
    pub hidden_by_variable: bool,
    /// `[matl_transmap]` texture: its values become the alpha channel of the material's texture.
    pub trans_map: Option<String>,
    /// Ready-made image (rendered text) used instead of loading `texture`.
    pub image: Option<Handle<Image>>,
    /// `[matl_lightmap]` texture and the script variable that switches it. The texture becomes
    /// the material's emissive texture with an emissive strength of 0; the variable is reported
    /// in [`ModelPart::light_variable`] for the runtime to drive (`crate::vehicle_lights`).
    pub light_map: Option<LightMap>,
}

/// One renderable submesh of a model: geometry bound to a single material.
#[derive(Debug, Clone)]
pub struct ModelPart {
    /// Mesh in Bevy (right-handed, Y-up) space.
    pub mesh: Handle<Mesh>,
    /// Material with the resolved texture.
    pub material: Handle<StandardMaterial>,
    /// Number of vertices in the mesh.
    pub vertices: usize,
    /// Script variable that scales the material's light map (`[matl_lightmap]`); `None` when the
    /// material has no light map or the map names no variable.
    pub light_variable: Option<String>,
    /// Translation of the mesh file's own `0x79` matrix (the modelling tool's pivot) in Bevy
    /// axes; `None` for files without one and for `.x` meshes (whose frames are applied).
    /// `[newanim] origin_from_mesh` rotates about it.
    pub pivot: Option<Vec3>,
}

/// Counters collected while loading, shown in the diagnostic overlay and logs.
#[derive(Debug, Clone, Default)]
pub struct ContentStats {
    /// Distinct `.o3d` files parsed.
    pub o3d_files: usize,
    /// Total vertices across all parsed `.o3d` files (counted once per file).
    pub o3d_vertices: usize,
    /// Distinct textures decoded.
    pub textures: usize,
    /// Meshes skipped because their vertex data is scrambled (author-protected files).
    pub protected_meshes: usize,
    /// Scrambled meshes replaced by a verified plain twin from another add-on.
    pub twin_meshes: usize,
    /// Scrambled meshes recovered by the confident geometric decoder.
    pub decoded_meshes: usize,
    /// Meshes left out for now because their background decoding is still running.
    pub deferred_meshes: usize,
    /// Meshes read with the axis-cycled layout (coherent positions, unusable stored normals).
    pub axis_cycled_meshes: usize,
    /// Human-readable descriptions of assets that failed to load.
    pub failures: Vec<String>,
}

/// The sampler of OMSI textures: repeating on all axes.
#[must_use]
pub fn repeat_sampler() -> ImageSampler {
    ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        address_mode_w: ImageAddressMode::Repeat,
        ..Default::default()
    })
}

/// Caches shared across a loading session.
#[derive(Default)]
pub struct ContentCache {
    pub(crate) textures: HashMap<PathBuf, Option<Handle<Image>>>,
    /// Textures without their alpha channel, by source image (see [`ContentLoader::opaque_variant`]).
    pub(crate) opaque: HashMap<bevy::asset::AssetId<Image>, Handle<Image>>,
    pub(crate) parts: HashMap<(PathBuf, String), Vec<ModelPart>>,
    pub(crate) protected: std::collections::HashSet<PathBuf>,
    /// Parameters of the scrambled-mesh decoder.
    pub decode_params: opts_core::formats::o3d::descramble::DecodeParams,
    /// Background decoder; without it meshes are decoded on the calling thread.
    pub decode_pool: Option<std::sync::Arc<crate::descramble_cache::DecodePool>>,
    /// Box the meshes loaded next are expected to lie in (the owning vehicle's bounding box);
    /// restricts the scrambled-mesh decoder.
    pub expected_bounds: Option<opts_core::formats::o3d::descramble::ExpectedBounds>,
    /// Repaint: texture file name (lower case) → replacement file, applied to the meshes loaded
    /// next (see `docs/src/formats/text/repaints.md`).
    pub texture_replacements: HashMap<String, PathBuf>,
    /// Meshes whose decoding was queued since the last [`Self::take_deferred`].
    pub deferred: std::collections::HashSet<crate::descramble_cache::Key>,
    /// Loading statistics.
    pub stats: ContentStats,
}

impl ContentCache {
    /// Takes the keys of meshes that were queued for background decoding since the last call.
    pub fn take_deferred(&mut self) -> std::collections::HashSet<crate::descramble_cache::Key> {
        std::mem::take(&mut self.deferred)
    }
}

/// Borrowed view tying an installation, caches and Bevy asset stores together.
pub struct ContentLoader<'a> {
    /// Installation used to resolve paths.
    pub install: &'a OmsiInstall,
    /// Session caches and statistics.
    pub cache: &'a mut ContentCache,
    /// Mesh store.
    pub meshes: &'a mut Assets<Mesh>,
    /// Material store.
    pub materials: &'a mut Assets<StandardMaterial>,
    /// Image store.
    pub images: &'a mut Assets<Image>,
}

impl ContentLoader<'_> {
    /// The texture without its alpha channel.
    ///
    /// OMSI uses a texture's alpha only where the material asks for it (`[matl_alpha]`,
    /// `[matl_transmap]`); paint textures such as `GN92_1.tga` carry unrelated data there (here
    /// almost all zeros), which made whole bodies invisible when blended. Images that are already
    /// opaque, or compressed, are returned unchanged.
    fn opaque_variant(&mut self, handle: &Handle<Image>) -> Handle<Image> {
        if let Some(h) = self.cache.opaque.get(&handle.id()) {
            return h.clone();
        }
        let out = match self.images.get(handle) {
            Some(img)
                if img.texture_descriptor.format
                    == bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb
                    && img
                        .data
                        .as_ref()
                        .is_some_and(|d| d.chunks_exact(4).any(|p| p[3] != 255)) =>
            {
                let mut copy = img.clone();
                if let Some(d) = copy.data.as_mut() {
                    for p in d.chunks_exact_mut(4) {
                        p[3] = 255;
                    }
                }
                self.images.add(copy)
            }
            _ => handle.clone(),
        };
        self.cache.opaque.insert(handle.id(), out.clone());
        out
    }

    /// Finds a verified plain twin of a scrambled mesh (see [`find_plain_twin`]).
    fn plain_twin(&self, path: &Path, scrambled: &[O3d]) -> Option<(Vec<O3d>, PathBuf)> {
        find_plain_twin(self.install, path, scrambled)
    }

    fn fail(&mut self, what: String) {
        tracing::error!(at = %crate::position_tag::position_tag(), "{what}");
        self.cache.stats.failures.push(what);
    }

    /// Loads a texture, searching `dirs` in order and then the global `Texture` folder.
    pub fn texture(&mut self, name: &str, dirs: &[PathBuf]) -> Option<Handle<Image>> {
        let name = name.trim();
        // Authors write `------` for "no texture"; that is not a missing file.
        if name.is_empty() || name.chars().all(|c| c == '-') {
            return None;
        }
        let global = self.install.root().join("Texture");
        let mut search: Vec<&Path> = dirs.iter().map(PathBuf::as_path).collect();
        search.push(&global);
        search.push(self.install.root());
        // Materials often name a texture by an extension that does not exist on disk
        // (`textures.md`): fall back to the same stem with the other image extensions.
        let stem = name.rsplit_once('.').map_or(name, |(s, _)| s);
        let candidates = std::iter::once(name.to_owned()).chain(
            ["dds", "bmp", "tga", "png", "jpg"]
                .iter()
                .map(|e| format!("{stem}.{e}")),
        );
        let mut found = None;
        'outer: for cand in candidates {
            for dir in &search {
                if let Some(p) = self.install.find_in(dir, &cand) {
                    found = Some(p);
                    break 'outer;
                }
            }
        }
        let found = found.or_else(|| {
            let near = dirs.first().map_or_else(|| self.install.root().to_path_buf(), Clone::clone);
            let hit = self.install.find_by_name(name, &near, true);
            if let Some(p) = &hit {
                tracing::debug!(texture = name, found = %p.display(), "texture rerouted to a same-named file");
            }
            hit
        });
        let Some(path) = found else {
            // Not real files: `\S:n` style names are script/slot references and `Textfeld_*`
            // textures are text fields OMSI renders at run time.
            if name.starts_with('\\') || name.to_lowercase().starts_with("textfeld") {
                tracing::debug!(
                    texture = name,
                    "runtime-generated texture reference (not a file)"
                );
                return None;
            }
            self.fail(format!(
                "texture not found: {name} (searched {} folders)",
                search.len()
            ));
            return None;
        };
        if let Some(cached) = self.cache.textures.get(&path) {
            return cached.clone();
        }
        let handle = match std::fs::read(&path) {
            Ok(bytes) => match load_bevy_image(&bytes) {
                Ok(mut image) => {
                    image.sampler = repeat_sampler();
                    self.cache.stats.textures += 1;
                    tracing::debug!(path = %path.display(), "loaded texture");
                    Some(self.images.add(image))
                }
                Err(e) => {
                    self.fail(format!("texture decode failed {}: {e}", path.display()));
                    None
                }
            },
            Err(e) => {
                self.fail(format!("texture read failed {}: {e}", path.display()));
                None
            }
        };
        self.cache.textures.insert(path, handle.clone());
        handle
    }

    /// Loads and splits an `.o3d` file into one [`ModelPart`] per used material.
    ///
    /// `tex_dirs` are searched for textures before the global `Texture` folder.
    pub fn o3d_parts(
        &mut self,
        o3d_path: &Path,
        tex_dirs: &[PathBuf],
        overrides: &[TextureOverride],
    ) -> Vec<ModelPart> {
        self.o3d_parts_impl(o3d_path, tex_dirs, overrides, false)
    }

    /// Like [`Self::o3d_parts`], but keeps the bone-weight vertex attributes (for meshes that
    /// are spawned with a `SkinnedMesh`, i.e. humans).
    pub fn o3d_skinned_parts(
        &mut self,
        o3d_path: &Path,
        tex_dirs: &[PathBuf],
        overrides: &[TextureOverride],
    ) -> Vec<ModelPart> {
        self.o3d_parts_impl(o3d_path, tex_dirs, overrides, true)
    }

    fn o3d_parts_impl(
        &mut self,
        o3d_path: &Path,
        tex_dirs: &[PathBuf],
        overrides: &[TextureOverride],
        keep_skin: bool,
    ) -> Vec<ModelPart> {
        let mut repaint: Vec<_> = self.cache.texture_replacements.iter().collect();
        repaint.sort();
        let key = (
            o3d_path.to_path_buf(),
            format!("{overrides:?}{keep_skin}{repaint:?}"),
        );
        if let Some(parts) = self.cache.parts.get(&key) {
            if self.cache.protected.contains(o3d_path) {
                self.cache.stats.protected_meshes += 1;
            }
            return parts.clone();
        }

        let bytes = match std::fs::read(o3d_path) {
            Ok(b) => b,
            Err(e) => {
                self.fail(format!("o3d read failed {}: {e}", o3d_path.display()));
                return Vec::new();
            }
        };
        let mut models: Vec<O3d> = match crate::mesh::parse_mesh_file(o3d_path, &bytes) {
            Ok(m) => m,
            Err(e) => {
                self.fail(format!("mesh parse failed {}: {e}", o3d_path.display()));
                return Vec::new();
            }
        };
        let is_o3d = o3d_path
            .extension()
            .is_some_and(|e| e.to_string_lossy().eq_ignore_ascii_case("o3d"));
        let mut deferred_here = false;
        if is_o3d {
            let unreadable = models
                .iter()
                .any(|m| matches!(m.vertex_layout(), o3d::VertexLayout::Unreadable));
            // Experimental: `OPTS_DECODE_SCRAMBLED=1` runs the geometric descrambler. It recovers
            // all vertices of only ~42 % of scrambled meshes and cannot tell which (texture
            // jitter stays), so it is off by default (`o3d.md`, Addendum 4).
            let experimental = unreadable && std::env::var_os("OPTS_DECODE_SCRAMBLED").is_some();
            if experimental {
                for m in &mut models {
                    let report = o3d::descramble::descramble(m);
                    tracing::warn!(
                        path = %o3d_path.display(),
                        accepted = report.accepted,
                        kept = report.kept_faces,
                        total = report.total_faces,
                        "scrambled mesh decoded heuristically (experimental)"
                    );
                }
            }
            // A scrambled mesh often exists unscrambled in another add-on (same name, same
            // counts, same coordinates): use that exact geometry instead.
            if unreadable && !experimental {
                if let Some((twin, from)) = self.plain_twin(o3d_path, &models) {
                    tracing::info!(
                        path = %o3d_path.display(),
                        twin = %from.display(),
                        "scrambled mesh replaced by its plain twin"
                    );
                    self.cache.stats.twin_meshes += 1;
                    models = twin;
                }
            }
            // No twin: try the confident decoder (two independent annealing runs must agree),
            // reusing earlier outcomes from the on-disk cache. `OPTS_NO_DESCRAMBLE=1` disables it.
            if unreadable && !experimental && std::env::var_os("OPTS_NO_DESCRAMBLE").is_none() {
                let fingerprint = self.cache.decode_params.fingerprint();
                deferred_here = false;
                for (mi, m) in models.iter_mut().enumerate() {
                    if !matches!(m.vertex_layout(), o3d::VertexLayout::Unreadable) {
                        continue;
                    }
                    let bounds = self.cache.expected_bounds;
                    let ckey = crate::descramble_cache::key_bounded(&bytes, mi, bounds);
                    let cached = self
                        .cache
                        .decode_pool
                        .as_ref()
                        .and_then(|p| p.result(ckey))
                        .or_else(|| crate::descramble_cache::load(ckey, fingerprint));
                    let applied = match &cached {
                        Some(c) => crate::descramble_cache::apply(m, c),
                        None => {
                            if let Some(pool) = &self.cache.decode_pool {
                                // Decode in the background; the tile is rebuilt when it is done.
                                let siblings: Vec<PathBuf> = o3d_path
                                    .file_name()
                                    .map(|n| self.install.files_named(&n.to_string_lossy()))
                                    .unwrap_or_default()
                                    .into_iter()
                                    .filter(|p| p != o3d_path)
                                    .collect();
                                pool.submit(ckey, m, siblings, bounds);
                                self.cache.deferred.insert(ckey);
                                deferred_here = true;
                                continue;
                            }
                            let before = m.clone();
                            let t = std::time::Instant::now();
                            let report = match bounds {
                                Some(b) => o3d::descramble::decode_confident_bounded(
                                    m,
                                    &self.cache.decode_params,
                                    b,
                                ),
                                None => o3d::descramble::decode_confident_with(
                                    m,
                                    &self.cache.decode_params,
                                ),
                            };
                            tracing::info!(
                                path = %o3d_path.display(),
                                accepted = report.accepted,
                                agreement = report.agreement,
                                coherence = report.coherence,
                                dropped_faces = report.dropped_faces,
                                uv_unresolved = report.uv.unresolved,
                                ms = t.elapsed().as_millis() as u64,
                                "scrambled mesh decoded"
                            );
                            let outcome = if report.accepted {
                                crate::descramble_cache::capture(&before, m)
                            } else {
                                crate::descramble_cache::Cached::Rejected
                            };
                            crate::descramble_cache::store(ckey, fingerprint, &outcome);
                            report.accepted
                        }
                    };
                    if applied {
                        self.cache.stats.decoded_meshes += 1;
                    }
                }
            }
            let unreadable = unreadable
                && !experimental
                && models
                    .iter()
                    .any(|m| matches!(m.vertex_layout(), o3d::VertexLayout::Unreadable));
            if unreadable && !experimental && deferred_here {
                // Decoding runs in the background; the caller rebuilds the object when it ends.
                self.cache.stats.deferred_meshes += 1;
                return Vec::new();
            }
            if unreadable && !experimental {
                // Vertex positions are permuted per vertex; the geometry cannot be reconstructed.
                self.cache.stats.protected_meshes += 1;
                self.cache.protected.insert(o3d_path.to_path_buf());
                tracing::warn!(
                    path = %o3d_path.display(),
                    "mesh vertex positions are scrambled; skipped"
                );
                self.cache.parts.insert(key, Vec::new());
                return Vec::new();
            }
            // A mesh that is hundreds of metres across is not an object: positions are corrupt
            // in a way the coherence measures did not catch (stray far-away vertices).
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
            let limit = if models.iter().any(|m| m.encryption_key.is_some()) {
                MAX_PLAUSIBLE_EXTENT_M
            } else {
                MAX_PLAUSIBLE_PLAIN_EXTENT_M
            };
            if extent > limit {
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

        let mut parts = Vec::new();
        for model in &models {
            for (slot, mut mesh) in crate::mesh::o3d_to_bevy_meshes_by_material(model) {
                // Static model parts carry no skeleton: joint attributes without a `SkinnedMesh`
                // component make the render pipeline layouts disagree (fatal wgpu validation error).
                if !keep_skin {
                    mesh.remove_attribute(Mesh::ATTRIBUTE_JOINT_INDEX);
                    mesh.remove_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT);
                }
                let vertices = mesh.count_vertices();
                let base = model
                    .materials
                    .get(slot)
                    .cloned()
                    .unwrap_or_else(|| o3d::Material {
                        diffuse: [1.0; 4],
                        specular: [0.0; 3],
                        emissive: [0.0; 3],
                        power: 0.0,
                        textures: Vec::new(),
                    });
                let over = find_override(&model.materials, slot, overrides);
                if over.is_some_and(|o| o.hidden_by_variable) {
                    continue;
                }
                let failures_before = self.cache.stats.failures.len();
                let tex_name = over
                    .map(|o| o.texture.clone())
                    .or_else(|| base.primary_texture().map(str::to_owned));
                let mut tex = match over.and_then(|o| o.image.clone()) {
                    Some(h) => Some(h),
                    None => tex_name.clone().and_then(|n| {
                        let replacement =
                            self.cache.texture_replacements.get(&base_name(&n)).cloned();
                        match replacement {
                            Some(path) => {
                                let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
                                let file = path
                                    .file_name()
                                    .map(|f| f.to_string_lossy().into_owned())
                                    .unwrap_or_default();
                                self.texture(&file, &[dir])
                                    .or_else(|| self.texture(&n, tex_dirs))
                            }
                            None => self.texture(&n, tex_dirs),
                        }
                    }),
                };
                // Benchmark sentinel (M-0054): the diffuse texture or the material slot is missing.
                let sentinel = crate::texture_sentinel::missing_texture_sentinel()
                    && (self.cache.stats.failures.len() > failures_before
                        || model.materials.get(slot).is_none());
                let mut blended_by_map = false;
                if let (Some(base_h), Some(tm)) =
                    (tex.clone(), over.and_then(|o| o.trans_map.clone()))
                {
                    if let Some(trans_h) = self.texture(&tm, tex_dirs) {
                        let composed = match (self.images.get(&base_h), self.images.get(&trans_h)) {
                            (Some(b), Some(t)) => compose_alpha(b, t),
                            _ => None,
                        };
                        if let Some(img) = composed {
                            tex = Some(self.images.add(img));
                            blended_by_map = true;
                        }
                    }
                }
                if !crate::material::uses_texture_alpha(over.and_then(|o| o.alpha))
                    && !blended_by_map
                {
                    tex = tex.map(|h| self.opaque_variant(&h));
                }
                let textured = tex.is_some();
                let mut material = o3d_material_to_standard_material(&base, tex);
                if textured {
                    // A textured surface is lit by its texture; the .o3d diffuse tint is a
                    // fixed-function multiplier that is white in practice.
                    material.base_color = bevy::color::Color::srgba(1.0, 1.0, 1.0, base.diffuse[3]);
                }
                if let Some(mode) = over
                    .and_then(|o| o.alpha)
                    .filter(|m| crate::material::uses_texture_alpha(Some(*m)))
                {
                    // Mode 2 blends (glass, smoke); anything else is a hard cutout (foliage, fences).
                    material.alpha_mode = if mode == 2 {
                        AlphaMode::Blend
                    } else {
                        AlphaMode::Mask(0.5)
                    };
                }
                if blended_by_map && over.and_then(|o| o.alpha).is_none() {
                    material.alpha_mode = AlphaMode::Blend;
                }
                // OMSI renders with back-face culling; keeping it also lets a camera that ends up
                // inside a building see out through the (culled) inner faces.
                material.cull_mode = Some(bevy::render::render_resource::Face::Back);
                material.double_sided = false;
                // A light map is emissive light switched by a script variable; it starts dark.
                // Maps without a variable have no switch and stay off ❓.
                let mut light_variable = None;
                if let Some(lm) = over
                    .and_then(|o| o.light_map.as_ref())
                    .filter(|lm| !lm.variable.trim().is_empty())
                {
                    if let Some(h) = self.texture(&lm.texture, tex_dirs) {
                        material.emissive_texture = Some(h);
                        material.emissive = bevy::color::LinearRgba::BLACK;
                        light_variable = Some(lm.variable.trim().to_owned());
                    }
                }
                if sentinel {
                    material = crate::texture_sentinel::sentinel_material();
                }
                tracing::debug!(path = %o3d_path.display(), slot, vertices, textured, alpha = ?material.alpha_mode, texture = ?tex_name, "model part");
                let pivot = model
                    .transform
                    .filter(|_| model.version != 0)
                    .map(|m| Vec3::new(m[3][0], m[3][1], -m[3][2]));
                parts.push(ModelPart {
                    mesh: self.meshes.add(mesh),
                    material: self.materials.add(material),
                    vertices,
                    light_variable,
                    pivot,
                });
            }
        }
        self.cache.parts.insert(key, parts.clone());
        parts
    }
}

/// Finds a verified plain twin of a scrambled mesh: same file name, same vertex and face
/// counts and the same multiset of (axis-order independent) position triples.
///
/// Public so the scenery benchmark (M-0054) predicts the geometry the loader will draw.
#[must_use]
pub fn find_plain_twin(
    install: &OmsiInstall,
    path: &Path,
    scrambled: &[O3d],
) -> Option<(Vec<O3d>, PathBuf)> {
    let [target] = scrambled else { return None };
    let name = path.file_name()?.to_string_lossy().into_owned();
    let want = position_signature(target);
    for cand in install.files_named(&name) {
        if cand == path {
            continue;
        }
        let Ok(bytes) = std::fs::read(&cand) else {
            continue;
        };
        let Ok(o) = o3d::parse(&bytes) else { continue };
        if o.encryption_key.is_none()
            && o.vertices.len() == target.vertices.len()
            && o.indices.len() == target.indices.len()
            && position_signature(&o) == want
        {
            return Some((vec![o], cand));
        }
    }
    None
}

/// Sorted multiset of per-vertex sorted coordinate triples (2/10 mm quantised): equal for a
/// mesh and its scrambled copy because scrambling only permutes components within a vertex.
fn position_signature(o: &O3d) -> Vec<[i32; 3]> {
    let mut v: Vec<[i32; 3]> = o
        .vertices
        .iter()
        .map(|v| {
            let mut a = [v.position.x, v.position.y, v.position.z];
            a.sort_by(f32::total_cmp);
            a.map(|x| (x * 10_000.0).round() as i32)
        })
        .collect();
    v.sort_unstable();
    v
}

fn base_name(path: &str) -> String {
    path.replace('\\', "/")
        .rsplit('/')
        .next()
        .unwrap_or("")
        .to_lowercase()
}

/// Finds the `[matl]` entry that applies to material `slot` of a mesh.
///
/// `[matl] <texture> <n>` names the **native texture** of the material it modifies and `n`
/// selects the n-th material (0-based) that uses that texture. This is how one mesh can carry
/// several overlay slots with the same texture (dirt, rain) and how a glass slot gets its own
/// alpha settings (measured on 275,721 entries: 65 % are consistent with this reading, 38 % with
/// "`n` is the slot index"). When the named texture matches no native slot, the entry is a
/// texture *replacement* and `n` is the slot index.
pub fn find_override<'a>(
    materials: &[o3d::Material],
    slot: usize,
    overrides: &'a [TextureOverride],
) -> Option<&'a TextureOverride> {
    let native: Vec<String> = materials
        .iter()
        .map(|m| base_name(m.primary_texture().unwrap_or("")))
        .collect();
    let name = native.get(slot)?;
    let occurrence = native[..slot].iter().filter(|n| *n == name).count();
    overrides
        .iter()
        .find(|o| o.slot == occurrence && base_name(&o.texture) == *name)
        .or_else(|| {
            overrides
                .iter()
                .find(|o| o.slot == slot && !native.contains(&base_name(&o.texture)))
        })
}

/// Spawns `parts` as children of `parent`, one `PbrBundle` each.
pub fn attach_model_parts(
    commands: &mut Commands,
    parent: Entity,
    parts: &[ModelPart],
    name: &str,
) -> Vec<Entity> {
    let mut spawned = Vec::with_capacity(parts.len());
    for (i, part) in parts.iter().enumerate() {
        let child = commands
            .spawn((
                Mesh3d(part.mesh.clone()),
                MeshMaterial3d(part.material.clone()),
            ))
            .insert(Name::new(format!("{name}_{i}")))
            .id();
        commands.entity(parent).add_child(child);
        spawned.push(child);
    }
    spawned
}

/// Returns `base` with its alpha channel replaced by the values of `trans` (nearest-neighbour
/// resampled). Works for RGBA8 bases and R8 / RGBA8 transparency maps; `None` otherwise.
fn compose_alpha(base: &Image, trans: &Image) -> Option<Image> {
    use bevy::render::render_resource::TextureFormat as F;
    let (bw, bh) = (base.width() as usize, base.height() as usize);
    let (tw, th) = (trans.width() as usize, trans.height() as usize);
    let base_data = base.data.as_deref()?;
    let trans_data = trans.data.as_deref()?;
    if !matches!(
        base.texture_descriptor.format,
        F::Rgba8UnormSrgb | F::Rgba8Unorm
    ) || base_data.len() != bw * bh * 4
        || tw == 0
        || th == 0
    {
        return None;
    }
    let sample: Box<dyn Fn(usize) -> u8> = match trans.texture_descriptor.format {
        F::R8Unorm if trans_data.len() == tw * th => Box::new(|i| trans_data[i]),
        F::Rgba8UnormSrgb | F::Rgba8Unorm if trans_data.len() == tw * th * 4 => {
            // Use the alpha channel if the map has one, else its luminance.
            let has_alpha = trans_data.chunks_exact(4).any(|p| p[3] != 255);
            Box::new(move |i| {
                let p = &trans_data[i * 4..i * 4 + 4];
                if has_alpha {
                    p[3]
                } else {
                    p[0]
                }
            })
        }
        _ => return None,
    };
    let mut out = base.clone();
    let out_data = out.data.as_mut()?;
    for y in 0..bh {
        for x in 0..bw {
            let (tx, ty) = (x * tw / bw, y * th / bh);
            out_data[(y * bw + x) * 4 + 3] = sample(ty * tw + tx);
        }
    }
    Some(out)
}
