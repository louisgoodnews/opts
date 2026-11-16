//! Textures named by a script string variable (`[matl_freetex]`).
//!
//! A material can show a texture whose file the vehicle's scripts choose at run time: the MB C2's
//! instrument display shows the page `c2step3_background` names (`Step3_Display\Tourdaten.png`
//! and others), the default texture is only the boot screen. [`FreeTextureBinding`] watches the
//! variable and gives the part its own material with that texture; a light map of the material
//! lights the new texture as it lit the old one (`crate::model::compose_light`).

use std::collections::HashMap;
use std::path::PathBuf;

use bevy::prelude::*;

use crate::VehicleScripts;

/// A vehicle part whose texture follows a string variable.
#[derive(Component, Debug, Clone)]
pub struct FreeTextureBinding {
    /// The vehicle root whose scripts own the variable.
    pub owner: Entity,
    /// The string variable naming the texture file (relative to a texture folder; backslashes
    /// and any letter case as in the scripts).
    pub variable: String,
    /// The folders the file is looked for in (the vehicle's texture folders).
    pub dirs: Vec<PathBuf>,
    /// The material's light map, which lights the shown texture.
    pub light_map: Option<Handle<Image>>,
    /// The name last applied.
    pub last: String,
    /// The material is this part's own copy.
    pub private: bool,
}

impl FreeTextureBinding {
    /// Binds `variable` of `owner`; files are looked for in `dirs`.
    #[must_use]
    pub fn new(
        owner: Entity,
        variable: String,
        dirs: Vec<PathBuf>,
        light_map: Option<Handle<Image>>,
    ) -> Self {
        Self {
            owner,
            variable,
            dirs,
            light_map,
            last: String::new(),
            private: false,
        }
    }
}

/// Loaded textures by file and lit textures by (file, light map), shared by all parts.
#[derive(Default)]
pub struct FreeTextureCache {
    loaded: HashMap<PathBuf, Option<Handle<Image>>>,
    lit: HashMap<(PathBuf, AssetId<Image>), Handle<Image>>,
}

/// Applies the texture each bound variable names, when the name changes.
pub fn free_texture_system(
    mut parts: Query<(
        &mut FreeTextureBinding,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
    vehicles: Query<&VehicleScripts>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut cache: Local<FreeTextureCache>,
) {
    for (mut binding, mut material) in &mut parts {
        let name = vehicles
            .get(binding.owner)
            .ok()
            .and_then(|s| {
                s.engine
                    .lock()
                    .ok()
                    .map(|e| e.get_string(&binding.variable).to_owned())
            })
            .unwrap_or_default();
        if name.trim().is_empty() || name == binding.last {
            continue;
        }
        binding.last.clone_from(&name);
        let Some(path) = binding
            .dirs
            .iter()
            .find_map(|d| crate::script_text::resolve_path(d, name.trim()))
        else {
            tracing::warn!(variable = %binding.variable, texture = %name, "free texture not found");
            continue;
        };
        let texture = cache
            .loaded
            .entry(path.clone())
            .or_insert_with(|| {
                let mut img = std::fs::read(&path)
                    .ok()
                    .and_then(|b| crate::texture::load_bevy_image(&b).ok())?;
                crate::texture_filter::prepare_object_texture(&mut img);
                Some(images.add(img))
            })
            .clone();
        let Some(texture) = texture else {
            continue;
        };
        let lit = binding.light_map.as_ref().and_then(|lm| {
            let key = (path.clone(), lm.id());
            if let Some(done) = cache.lit.get(&key) {
                return Some(done.clone());
            }
            let composed = crate::model::compose_light(images.get(&texture)?, images.get(lm)?)?;
            let done = images.add(composed);
            cache.lit.insert(key, done.clone());
            Some(done)
        });
        if !binding.private {
            let Some(copy) = materials.get(&material.0).cloned() else {
                continue;
            };
            material.0 = materials.add(copy);
            binding.private = true;
        }
        tracing::debug!(variable = %binding.variable, texture = %path.display(), lit = lit.is_some(), "free texture applied");
        if let Some(mut m) = materials.get_mut(&material.0) {
            // A self-illuminating part shows its picture as its glow (`material::emission_texture`).
            let glows_with_picture =
                m.emissive_texture.is_some() && m.emissive_texture == m.base_color_texture;
            m.base_color_texture = Some(texture.clone());
            if lit.is_some() {
                m.emissive_texture = lit;
            } else if glows_with_picture {
                m.emissive_texture = Some(texture);
            }
        }
    }
}

#[cfg(test)]
mod tests;
