//! Runtime loading of OMSI models and textures from an installation directory.
//!
//! [`ContentLoader`] turns a mesh file plus its material/texture overrides (from a `.sco` or a
//! vehicle `model.cfg`) into [`ModelPart`]s: one mesh per material slot, with textures resolved
//! through [`crate::install::OmsiInstall`]. It reuses the crate's mesh, material and texture converters and
//! is shared by scenery objects, splines and vehicles.

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

use crate::material::o3d_material_to_standard_material;

/// Meshes larger than this along any axis are rejected as corrupt (metres). Road and crossing
/// models are legitimately hundreds of metres long and skyline backdrops kilometres (up to
/// 3,200 m observed); keyed meshes are decoded exactly, so they get no stricter limit than plain
/// ones (the 400 m limit of the heuristic decoder skipped 5 city blocks of `DBC_Map` and the
/// Hamburg backdrops, M-0066).
pub const MAX_PLAUSIBLE_EXTENT_M: f32 = 3000.0;

mod cache;
mod o3d_loading;
mod overrides;
mod texture_load;
mod types;

pub use cache::{ContentCache, ContentLoader};
pub use overrides::compose_light;
use overrides::{base_name, compose_alpha, transmap_glass};
pub use overrides::{
    find_override, is_weather_overlay, overlay_class, transmap_glass_rgba, OverlayClass,
    TextureOverride, TRANSMAP_OPAQUE, TRANSMAP_SPLIT_SHARE,
};
pub use types::{ContentStats, ModelPart};

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
