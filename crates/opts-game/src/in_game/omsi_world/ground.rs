//! Painted ground: layered tile materials and tile origins.

use bevy::prelude::*;
use opts_bevy::ground_material::GroundMaterial;
use opts_bevy::{terrain_material, ContentLoader, TILE_SIZE_M};

/// Asset stores the loading systems write to (grouped to stay within Bevy's parameter limit).
use super::WorldLoadJob;

pub(super) fn tile_origin(key: (i32, i32)) -> (f32, f32) {
    (key.0 as f32 * TILE_SIZE_M, key.1 as f32 * TILE_SIZE_M)
}

/// Layer textures of a map shared by all its tiles (one GPU array with mip chains).
pub(super) struct GroundArray {
    layers: Handle<Image>,
    /// Repeats per tile edge of each array layer.
    repeats: Vec<f32>,
    /// Whether the layer's texture decoded; layers that did not are never blended in.
    usable: Vec<bool>,
}

/// Builds the layered ground material of a tile from its layer masks; `None` if the tile has
/// none (the caller then repeats the base texture) or the base layer cannot be read.
pub(super) fn compose_tile_material(
    job: &mut WorldLoadJob,
    key: (i32, i32),
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
    ground_materials: &mut Assets<GroundMaterial>,
) -> Option<Handle<GroundMaterial>> {
    use opts_bevy::ground::CpuImage;
    use opts_bevy::ground_material::{
        ground_extension, layer_array, mask_array, MAX_GROUND_LAYERS,
    };
    let file = job.tile_files.get(&key)?.clone();
    let layer_count = job.manifest.ground_textures.len().min(MAX_GROUND_LAYERS);
    let mut masks: Vec<Option<CpuImage>> = vec![None];
    for n in 1..layer_count {
        let mask = [
            format!("texture/map/{file}.{n}.dds"),
            format!("texture/{file}.{n}.dds"),
        ]
        .iter()
        .find_map(|rel| job.install.find_in(&job.config.map_dir, rel))
        .inspect(|p| {
            job.loading_deps
                .insert(opts_bevy::hot_reload::cache::normalize(p));
        })
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| opts_bevy::load_bevy_image(&b).ok())
        .and_then(|img| CpuImage::from_image(&img));
        masks.push(mask);
    }
    if masks.iter().all(Option::is_none) {
        return None;
    }
    if job.ground_array.is_none() {
        // Decode every layer once per map and upload them as one mipmapped array.
        let dirs = vec![
            job.config.map_dir.clone(),
            job.config.map_dir.join("texture"),
        ];
        let mut decoded: Vec<Option<CpuImage>> = Vec::new();
        for n in 0..layer_count {
            let name = job.manifest.ground_textures[n].ground_texture.clone();
            let img = {
                let mut loader = ContentLoader {
                    install: &job.install,
                    cache: &mut job.cache,
                    meshes,
                    materials,
                    images,
                };
                loader.texture(&name, &dirs)
            }
            .and_then(|h| images.get(&h).and_then(CpuImage::from_image));
            if img.is_none() {
                tracing::warn!(layer = n, texture = %name, "ground layer texture cannot be decoded; layer skipped");
            }
            decoded.push(img);
        }
        let base = decoded.first().cloned().flatten()?;
        let size = decoded
            .iter()
            .flatten()
            .map(|d| d.width.max(d.height))
            .max()
            .unwrap_or(512)
            .clamp(256, 1024)
            .next_power_of_two();
        let refs: Vec<&CpuImage> = decoded
            .iter()
            .map(|d| d.as_ref().unwrap_or(&base))
            .collect();
        let layers = images.add(layer_array(&refs, size));
        job.ground_array = Some(GroundArray {
            layers,
            repeats: (0..layer_count)
                .map(|n| job.manifest.ground_textures[n].params[2].max(1.0))
                .collect(),
            usable: decoded.iter().map(Option::is_some).collect(),
        });
    }
    let array = job.ground_array.as_ref()?;
    if !array.usable.first().copied().unwrap_or(false) {
        return None;
    }
    let size = masks
        .iter()
        .flatten()
        .map(|m| m.width.max(m.height))
        .max()
        .unwrap_or(512)
        .clamp(256, 1024);
    let mask_refs: Vec<Option<&CpuImage>> = masks
        .iter()
        .enumerate()
        .map(|(n, m)| {
            m.as_ref()
                .filter(|_| array.usable.get(n).copied().unwrap_or(false))
        })
        .collect();
    let mask_handle = images.add(mask_array(&mask_refs, size));
    Some(ground_materials.add(GroundMaterial {
        base: terrain_material(None),
        extension: ground_extension(array.layers.clone(), mask_handle, &array.repeats),
    }))
}
