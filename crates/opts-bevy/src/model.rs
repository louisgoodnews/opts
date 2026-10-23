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

/// Meshes larger than this along any axis are rejected as corrupt (metres). Road and crossing
/// models are legitimately hundreds of metres long and skyline backdrops kilometres (up to
/// 3,200 m observed); keyed meshes are decoded exactly, so they get no stricter limit than plain
/// ones (the 400 m limit of the heuristic decoder skipped 5 city blocks of `DBC_Map` and the
/// Hamburg backdrops, M-0066).
pub const MAX_PLAUSIBLE_EXTENT_M: f32 = 3000.0;

mod overrides;
use overrides::{base_name, compose_alpha, transmap_glass};
pub use overrides::{
    find_override, is_static_overlay, transmap_glass_rgba, TextureOverride, TRANSMAP_OPAQUE,
    TRANSMAP_SPLIT_SHARE,
};

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
    /// Transmap materials split into an opaque body and a blended glass part.
    pub transmap_splits: usize,
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

/// A composition: base image, transparency map, and whether the glass pass is split off.
pub(crate) type ComposedKey = (
    bevy::asset::AssetId<Image>,
    bevy::asset::AssetId<Image>,
    bool,
);
/// Its result: the composed picture and the glass pass.
pub(crate) type ComposedImages = (Handle<Image>, Option<Handle<Image>>);

/// Caches shared across a loading session.
#[derive(Default)]
pub struct ContentCache {
    pub(crate) textures: HashMap<PathBuf, Option<Handle<Image>>>,
    /// Textures without their alpha channel, by source image (see [`ContentLoader::opaque_variant`]).
    pub(crate) opaque: HashMap<bevy::asset::AssetId<Image>, Handle<Image>>,
    /// Base texture composed with its transparency map (`[matl_transmap]`): the composed
    /// picture, and its glass pass when the composition is split. The composition decodes both
    /// images and builds a new mip chain (tens of milliseconds); every vehicle of a model family
    /// uses the same pairs, so it is done once.
    pub(crate) composed: HashMap<ComposedKey, ComposedImages>,
    /// The glass pass of a blended body texture, by source image (`None`: not mostly opaque).
    pub(crate) glass_split: HashMap<bevy::asset::AssetId<Image>, Option<Handle<Image>>>,
    pub(crate) parts: HashMap<(PathBuf, String), Vec<ModelPart>>,
    /// `[crossing_heightdeformation]` base surfaces by file.
    pub(crate) bases: HashMap<PathBuf, Option<std::sync::Arc<BaseSurface>>>,
    /// Shared handles by content (`OPTS_INSTANCING=dedupe`, see [`crate::instancing`]).
    pub pool: crate::instancing::AssetPool,
    /// Static parts of the tile being loaded (`OPTS_INSTANCING=merge`).
    pub merger: crate::instancing::StaticMerger,
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

/// `load_bevy_image` under the `tex.decode` stream-trace span (M-0089).
fn decode_traced(bytes: &[u8]) -> Result<Image, crate::texture::TextureError> {
    let _t = opts_logging::stream_trace::span("tex.decode");
    load_bevy_image(bytes)
}

mod o3d_loading;

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
        let t_read = opts_logging::stream_trace::span("tex.read");
        let read = std::fs::read(&path);
        drop(t_read);
        let handle = match read {
            Ok(bytes) => match decode_traced(&bytes) {
                Ok(mut image) => {
                    {
                        let _t = opts_logging::stream_trace::span("tex.prepare_mips");
                        crate::texture_filter::prepare_object_texture(&mut image);
                    }
                    self.cache.stats.textures += 1;
                    tracing::debug!(path = %path.display(), "loaded texture");
                    let _t = opts_logging::stream_trace::span("tex.assets_add");
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
}

mod trace;
use trace::TraceScratch;

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

#[cfg(test)]
mod alphascale_tests;

#[cfg(test)]
mod transmap_split_tests;

#[cfg(test)]
mod tests;
