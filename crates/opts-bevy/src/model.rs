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
use bevy::image::{Image, ImageSampler};
use bevy::math::Vec3;
use bevy::mesh::Mesh;
use bevy::mesh::Mesh3d;
use bevy::pbr::MeshMaterial3d;
use bevy::pbr::StandardMaterial;
use bevy::prelude::AlphaMode;

use opts_core::formats::o3d::{self, O3d};
use opts_core::formats::scenery::BaseSurface;
use opts_core::formats::vehicles::model::LightMap;

use crate::install::OmsiInstall;
use crate::material::o3d_material_to_standard_material;
use crate::texture::load_bevy_image;

/// Keyed (decoded) meshes larger than this along any axis are rejected as corrupt (metres).
pub const MAX_PLAUSIBLE_EXTENT_M: f32 = 400.0;

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
    /// The script variable that scales the material's alpha (`[alphascale]`). An overlay that uses
    /// alpha and whose variable OPTS never raises (rain, dirt) is not built at all, see
    /// [`TextureOverride::is_variable_hidden`]; any other alpha-scaled material is built and
    /// follows its variable at run time (`ModelPart::alpha_variable`).
    pub alpha_variable: Option<String>,
    /// `[matl_transmap]` texture: its values become the alpha channel of the material's texture.
    pub trans_map: Option<String>,
    /// Ready-made image (rendered text) used instead of loading `texture`.
    pub image: Option<Handle<Image>>,
    /// `[matl_lightmap]` texture and the script variable that switches it. The texture becomes
    /// the material's emissive texture with an emissive strength of 0; the variable is reported
    /// in [`ModelPart::light_variable`] for the runtime to drive (`crate::vehicle_lights`).
    pub light_map: Option<LightMap>,
}

impl TextureOverride {
    /// Whether the slot is an overlay that stays invisible because OPTS never raises its variable.
    ///
    /// `[alphascale]` also appears on opaque base materials (`Envir_Brightness` on the body of a
    /// Crealis bus, on the night-lit skyline blocks of the Chicago map): alpha-scaling something
    /// that does not use alpha is no reason to drop the surface, the whole body of such a vehicle
    /// vanished (P-1014, MI-0029). Only materials with a `[matl_alpha]` mode other than 0 or a
    /// `[matl_transmap]` texture are overlays.
    ///
    /// Of those, only the overlays of weather the simulation does not have ([`is_static_overlay`]:
    /// rain on the windows, dirt) are dropped at load. The others — glass scaled by
    /// `Envir_Brightness` (6,651 materials), `szyby`, `para_*` — belong on the vehicle and follow
    /// their variable (MAN SD83 without window glass, 2026-10-14).
    #[must_use]
    pub fn is_variable_hidden(&self) -> bool {
        self.alpha_variable
            .as_deref()
            .is_some_and(is_static_overlay)
            && (crate::material::uses_texture_alpha(self.alpha) || self.trans_map.is_some())
    }

    /// The variable the part follows at run time: set for an alpha-using material scaled by a
    /// variable that is not a static overlay.
    #[must_use]
    pub fn runtime_alpha_variable(&self) -> Option<&str> {
        self.alpha_variable
            .as_deref()
            .filter(|v| !is_static_overlay(v))
            .filter(|_| crate::material::uses_texture_alpha(self.alpha) || self.trans_map.is_some())
    }
}

/// Variables of weather the simulation does not have: their overlays (rain on windows, dirt) are
/// always 0 and not worth an entity (`Rain_Window_*_Wetness` 22,888 uses, `Dirt_*` 15,733 in the
/// installed models). When weather or dirt is simulated they stop being static.
#[must_use]
pub fn is_static_overlay(variable: &str) -> bool {
    let v = variable.trim().to_ascii_lowercase();
    ["rain_window_", "dirt_", "precip_"]
        .iter()
        .any(|p| v.starts_with(p))
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
    /// Script variable that scales the material's alpha (`[alphascale]`), for parts that follow it
    /// at run time (window glass); `None` for ordinary parts.
    pub alpha_variable: Option<String>,
    /// The ready-made image that replaced the material's texture (rendered text), if any.
    pub image: Option<Handle<Image>>,
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
    /// Meshes skipped because their geometry is unusable (a keyed file the generator does not
    /// cover, or an implausible extent).
    pub protected_meshes: usize,
    /// Keyed meshes decoded exactly by following the exporter's generator (`o3d::exact`).
    pub exact_meshes: usize,
    /// Meshes read with the axis-cycled layout (coherent positions, unusable stored normals).
    pub axis_cycled_meshes: usize,
    /// Material slots left out because a script variable scales their alpha (`[alphascale]`):
    /// an object whose slots are all of that kind has nothing to draw, which is not a load failure.
    pub hidden_parts: usize,
    /// Human-readable descriptions of assets that failed to load.
    pub failures: Vec<String>,
}

/// The sampler of OMSI textures: repeating on all axes, filtered per the "Texture filtering"
/// setting (see [`crate::texture_filter`]).
#[must_use]
pub fn repeat_sampler() -> ImageSampler {
    crate::texture_filter::current_object_sampler()
}

/// Key of the parts cache: the file and everything that changes what is built from it (the
/// material overrides, skinning, the repaint, bone slots and the crossing base it is bent over).
fn parts_key(
    o3d_path: &Path,
    overrides: &[TextureOverride],
    keep_skin: bool,
    repaint: &[(&String, &PathBuf)],
    slots: Option<&[(String, u16)]>,
    base: Option<&Path>,
) -> (PathBuf, String) {
    (
        o3d_path.to_path_buf(),
        format!("{overrides:?}{keep_skin}{repaint:?}{slots:?}{base:?}"),
    )
}

/// Reads a mesh file with positions as the exporter wrote them: keyed `.o3d` files are decoded
/// exactly (`o3d::exact`); a file that stays scrambled is an error.
///
/// # Errors
/// The file cannot be read, parsed or decoded.
pub fn load_decoded_meshes(path: &Path) -> Result<Vec<O3d>, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let mut models = crate::mesh::parse_mesh_file(path, &bytes)?;
    for m in &mut models {
        if m.encryption_key.is_some() {
            let _ = o3d::exact::decode(m);
        }
    }
    if models
        .iter()
        .any(|m| matches!(m.vertex_layout(), o3d::VertexLayout::Unreadable))
    {
        return Err("vertex positions are scrambled and could not be decoded".into());
    }
    Ok(models)
}

/// Caches shared across a loading session.
#[derive(Default)]
pub struct ContentCache {
    pub(crate) textures: HashMap<PathBuf, Option<Handle<Image>>>,
    /// Textures without their alpha channel, by source image (see [`ContentLoader::opaque_variant`]).
    pub(crate) opaque: HashMap<bevy::asset::AssetId<Image>, Handle<Image>>,
    pub(crate) parts: HashMap<(PathBuf, String), Vec<ModelPart>>,
    /// `[crossing_heightdeformation]` base surfaces by file.
    pub(crate) bases: HashMap<PathBuf, Option<std::sync::Arc<BaseSurface>>>,
    pub(crate) protected: std::collections::HashSet<PathBuf>,
    /// Repaint: texture file name (lower case) → replacement file, applied to the meshes loaded
    /// next (see `docs/src/formats/text/repaints.md`).
    pub texture_replacements: HashMap<String, PathBuf>,
    /// Loading statistics.
    pub stats: ContentStats,
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
                    crate::texture_filter::prepare_object_texture(&mut image);
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
            let limit = if models.iter().any(|m| m.encryption_key.is_some()) {
                MAX_PLAUSIBLE_EXTENT_M
            } else {
                MAX_PLAUSIBLE_PLAIN_EXTENT_M
            };
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

        let mut parts = Vec::new();
        for model in &models {
            for (slot, mut mesh) in crate::mesh::o3d_to_bevy_meshes_by_material_slots(model, slots)
            {
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
                if over.is_some_and(TextureOverride::is_variable_hidden) {
                    self.cache.stats.hidden_parts += 1;
                    sc.hidden += 1;
                    continue;
                }
                let failures_before = self.cache.stats.failures.len();
                // An entry without a texture (a `[matl_change]` or `[matl_item]` that only sets
                // switches) keeps the material's own texture.
                let tex_name = over
                    .map(|o| o.texture.clone())
                    .filter(|t| !t.trim().is_empty())
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
                if sc.lap.active() {
                    sc.materials.push(crate::scene_trace::MaterialNote {
                        slot,
                        texture: tex_name.clone().unwrap_or_default(),
                        alpha: match material.alpha_mode {
                            AlphaMode::Blend => "blend",
                            AlphaMode::Mask(_) => "mask",
                            _ => "opaque",
                        }
                        .into(),
                        textured,
                    });
                    if tex_name.is_some() && !textured {
                        sc.warnings.push(format!(
                            "texture not found: {}",
                            tex_name.clone().unwrap_or_default()
                        ));
                    }
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
                    alpha_variable: over
                        .and_then(|o| o.runtime_alpha_variable())
                        .map(str::to_owned),
                    image: over.and_then(|o| o.image.clone()),
                    pivot,
                });
            }
        }
        sc.build_ms = sc.lap.split();
        self.cache.parts.insert(key, parts.clone());
        parts
    }
}

/// What the scene trace learns while one mesh file loads (costs nothing when the trace is off:
/// `Lap` is inactive and no string is built).
struct TraceScratch {
    lap: crate::scene_trace::Lap,
    stage: &'static str,
    read_ms: f32,
    parse_ms: f32,
    decode_ms: f32,
    build_ms: f32,
    hidden: usize,
    materials: Vec<crate::scene_trace::MaterialNote>,
    warnings: Vec<String>,
}

impl TraceScratch {
    fn new() -> Self {
        Self {
            lap: crate::scene_trace::Lap::start(),
            stage: "",
            read_ms: 0.0,
            parse_ms: 0.0,
            decode_ms: 0.0,
            build_ms: 0.0,
            hidden: 0,
            materials: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn warn(&mut self, text: impl FnOnce() -> String) {
        if self.lap.active() {
            self.warnings.push(text());
        }
    }

    /// Hands the finished note to the trace.
    fn finish(self, meshes: &Assets<Mesh>, path: &Path, parts: &[ModelPart]) {
        use crate::scene_trace as st;
        let had_note = st::has_note(path);
        if self.stage == "parts-cache" && had_note {
            return;
        }
        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
        let mut any = false;
        for p in parts {
            let Some(pos) = meshes
                .get(&p.mesh)
                .and_then(|m| m.attribute(Mesh::ATTRIBUTE_POSITION))
                .and_then(|a| a.as_float3())
            else {
                continue;
            };
            for v in pos {
                any = true;
                for i in 0..3 {
                    lo[i] = lo[i].min(v[i]);
                    hi[i] = hi[i].max(v[i]);
                }
            }
        }
        let mut warnings = self.warnings;
        if self.hidden > 0 && parts.is_empty() {
            warnings.clear();
        }
        let stage = if self.stage == "parts-cache" {
            "cached"
        } else if parts.is_empty() && self.hidden > 0 && self.stage == "plain" {
            "hidden"
        } else {
            self.stage
        };
        st::note_mesh(
            path,
            st::MeshNote {
                stage: stage.into(),
                read_ms: self.read_ms,
                parse_ms: self.parse_ms,
                decode_ms: self.decode_ms,
                build_ms: self.build_ms,
                vertices: parts.iter().map(|p| p.vertices).sum(),
                parts: parts.len(),
                materials: self.materials,
                warnings,
                bbox: any.then_some(st::TraceBounds { min: lo, max: hi }),
                ..st::MeshNote::default()
            },
        );
    }
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
/// resampled). Bases and maps may be RGBA8 or block-compressed (BC1-3); an R8 map is read directly.
/// Only level 0 of either input is read; the result gets a fresh mip chain like any loaded texture.
fn compose_alpha(base: &Image, trans: &Image) -> Option<Image> {
    // DDS textures stay block-compressed on the GPU; the texel values are needed here.
    let base = crate::texture::decode_to_rgba8(base)?;
    let trans_decoded = crate::texture::decode_to_rgba8(trans);
    let trans = trans_decoded.as_ref().unwrap_or(trans);
    let base = &base;
    use bevy::render::render_resource::TextureFormat as F;
    let (bw, bh) = (base.width() as usize, base.height() as usize);
    let (tw, th) = (trans.width() as usize, trans.height() as usize);
    let base_data = base.data.as_deref()?;
    let trans_data = trans.data.as_deref()?;
    if !matches!(
        base.texture_descriptor.format,
        F::Rgba8UnormSrgb | F::Rgba8Unorm
    ) || base_data.len() < bw * bh * 4
        || tw == 0
        || th == 0
    {
        return None;
    }
    let sample: Box<dyn Fn(usize) -> u8> = match trans.texture_descriptor.format {
        F::R8Unorm if trans_data.len() >= tw * th => Box::new(|i| trans_data[i]),
        F::Rgba8UnormSrgb | F::Rgba8Unorm if trans_data.len() >= tw * th * 4 => {
            // Use the alpha channel if the map has one, else its luminance.
            let has_alpha = trans_data[..tw * th * 4]
                .chunks_exact(4)
                .any(|p| p[3] != 255);
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
    out.texture_descriptor.mip_level_count = 1;
    let out_data = out.data.as_mut()?;
    out_data.truncate(bw * bh * 4);
    for y in 0..bh {
        for x in 0..bw {
            let (tx, ty) = (x * tw / bw, y * th / bh);
            out_data[(y * bw + x) * 4 + 3] = sample(ty * tw + tx);
        }
    }
    crate::texture_filter::prepare_object_texture(&mut out);
    Some(out)
}

#[cfg(test)]
mod alphascale_tests {
    use super::TextureOverride;

    fn over(alpha: Option<u32>, trans: Option<&str>, scaled: bool) -> TextureOverride {
        over_with(alpha, trans, scaled, "Dirt_Norm")
    }

    fn over_with(
        alpha: Option<u32>,
        trans: Option<&str>,
        scaled: bool,
        name: &str,
    ) -> TextureOverride {
        TextureOverride {
            slot: 0,
            texture: "t.tga".into(),
            alpha,
            alpha_variable: scaled.then(|| name.to_owned()),
            trans_map: trans.map(str::to_owned),
            light_map: None,
            image: None,
        }
    }

    /// MAN SD83: the window glass `Fenster.tga` is `[matl_alpha] 2` scaled by `Envir_Brightness`;
    /// it follows the variable at run time instead of being dropped. Weather overlays stay out.
    #[test]
    fn glass_scaled_by_a_live_variable_is_built_and_follows_it() {
        let glass = over_with(Some(2), None, true, "Envir_Brightness");
        assert!(!glass.is_variable_hidden());
        assert_eq!(glass.runtime_alpha_variable(), Some("Envir_Brightness"));
        for weather in ["Rain_Window_Norm_Wetness", "Dirt_Wiped", "Precip_Mirror"] {
            let o = over_with(Some(2), None, true, weather);
            assert!(o.is_variable_hidden(), "{weather}");
            assert_eq!(o.runtime_alpha_variable(), None);
        }
        // Script-set variables (`szyby`, `para_front`) follow the scripts.
        assert_eq!(
            over_with(Some(2), None, true, "szyby").runtime_alpha_variable(),
            Some("szyby")
        );
        // An opaque material is not an overlay, whatever its variable.
        assert_eq!(
            over_with(None, None, true, "Envir_Brightness").runtime_alpha_variable(),
            None
        );
    }

    /// MI-0029: `[alphascale] Envir_Brightness` on an opaque base material (380 materials of the
    /// corpus: Crealis bodies, the Chicago skyline) must not hide the surface; overlays that use
    /// alpha (rain, dirt, condensation: 43,968 materials) stay hidden by their variable.
    #[test]
    fn only_alpha_overlays_are_hidden_by_their_variable() {
        assert!(
            !over(None, None, true).is_variable_hidden(),
            "opaque body stays"
        );
        assert!(
            !over(Some(0), None, true).is_variable_hidden(),
            "mode 0 is opaque"
        );
        assert!(
            over(Some(2), None, true).is_variable_hidden(),
            "blended overlay"
        );
        assert!(
            over(Some(1), None, true).is_variable_hidden(),
            "cutout overlay"
        );
        assert!(
            over(None, Some("mask.bmp"), true).is_variable_hidden(),
            "transmap overlay"
        );
        assert!(
            !over(Some(2), None, false).is_variable_hidden(),
            "no variable: visible"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parts bent over a crossing base are cached apart from the flat parts and per base file.
    #[test]
    fn the_parts_cache_key_includes_the_crossing_base() {
        let file = Path::new("Kreuz.o3d");
        let flat = parts_key(file, &[], false, &[], None, None);
        let a = parts_key(file, &[], false, &[], None, Some(Path::new("a_base.o3d")));
        let b = parts_key(file, &[], false, &[], None, Some(Path::new("b_base.o3d")));
        assert_ne!(flat, a);
        assert_ne!(a, b);
        assert_eq!(
            a,
            parts_key(file, &[], false, &[], None, Some(Path::new("a_base.o3d")))
        );
    }
}
