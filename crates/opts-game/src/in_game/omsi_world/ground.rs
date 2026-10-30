//! Painted ground: layered tile materials.

use bevy::prelude::*;
use opts_bevy::ground_material::GroundMaterial;
use opts_bevy::{terrain_material, ContentLoader};

/// Asset stores the loading systems write to (grouped to stay within Bevy's parameter limit).
use super::WorldLoadJob;

/// Layer textures of a map shared by all its tiles (one GPU array with mip chains). Only the
/// layers some tile paints are in it (plus the base), so maps with many `[groundtex]` entries
/// stay within the array budget.
pub(super) struct GroundArray {
    layers: Handle<Image>,
    /// Detail textures (white where a layer has none), same layer order.
    details: Handle<Image>,
    /// Array index of each `[groundtex]` layer; `None` when no tile paints it or its texture
    /// cannot be decoded (such layers are never blended in).
    index: Vec<Option<u32>>,
}

/// The `[groundtex]` layers (`1..count`) that have a mask file in the map's `texture/map/` or
/// `texture/` folder (`<tile>.map.<n>.dds`, any case), ascending.
fn painted_layers(map_dir: &std::path::Path, count: usize) -> Vec<usize> {
    let mut found = std::collections::BTreeSet::new();
    let dirs = std::fs::read_dir(map_dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case("texture")
        })
        .flat_map(|tex| {
            let sub = std::fs::read_dir(tex.path())
                .into_iter()
                .flatten()
                .flatten()
                .filter(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case("map"))
                .map(|e| e.path());
            std::iter::once(tex.path()).chain(sub).collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    for dir in dirs {
        for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            let Some(stem) = name.strip_suffix(".dds") else {
                continue;
            };
            let Some((head, n)) = stem.rsplit_once('.') else {
                continue;
            };
            if head.ends_with(".map") {
                if let Ok(n) = n.parse::<usize>() {
                    if (1..count).contains(&n) {
                        found.insert(n);
                    }
                }
            }
        }
    }
    found.into_iter().collect()
}

/// The painted ground masks of a tile (`<tile>.<n>.dds`), by layer number.
fn read_masks(
    job: &mut WorldLoadJob,
    file: &str,
    layer_total: usize,
) -> Vec<(usize, opts_bevy::ground::CpuImage)> {
    use opts_bevy::ground::CpuImage;
    let mut masks: Vec<(usize, CpuImage)> = Vec::new();
    for n in 1..layer_total {
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
        if let Some(mask) = mask {
            masks.push((n, mask));
        }
    }
    masks
}

/// The base and the painted layers: (layer number, texture, detail texture). `None` when the base
/// layer cannot be decoded; another layer that cannot is skipped.
type DecodedLayers = Vec<(
    usize,
    opts_bevy::ground::CpuImage,
    Option<opts_bevy::ground::CpuImage>,
)>;

fn decode_layers(
    job: &mut WorldLoadJob,
    (meshes, materials, images): (
        &mut Assets<Mesh>,
        &mut Assets<StandardMaterial>,
        &mut Assets<Image>,
    ),
    (wanted, dirs): (&[usize], &[std::path::PathBuf]),
) -> Option<DecodedLayers> {
    use opts_bevy::ground::CpuImage;
    let mut decoded: DecodedLayers = Vec::new();
    for &n in wanted {
        let tex = &job.manifest.ground_textures[n];
        let (name, detail_name) = (tex.ground_texture.clone(), tex.detail_texture.clone());
        let mut loader = ContentLoader {
            install: &job.install,
            cache: &mut job.cache,
            meshes,
            materials,
            images,
        };
        let img = loader
            .texture(&name, dirs)
            .and_then(|h| loader.images.get(&h).and_then(CpuImage::from_image));
        let detail = if detail_name.trim().is_empty() {
            None
        } else {
            loader
                .texture(&detail_name, dirs)
                .and_then(|h| loader.images.get(&h).and_then(CpuImage::from_image))
        };
        match img {
            Some(img) => decoded.push((n, img, detail)),
            None if n == 0 => return None,
            None => {
                tracing::warn!(layer = n, texture = %name, "ground layer texture cannot be decoded; layer skipped");
            }
        }
    }
    Some(decoded)
}

/// Decodes the base and every painted layer once per map and uploads them as one mipmapped array
/// (`job.ground_array`). `None` when the base layer cannot be decoded.
fn build_ground_array(
    job: &mut WorldLoadJob,
    (images, materials, meshes): (
        &mut Assets<Image>,
        &mut Assets<StandardMaterial>,
        &mut Assets<Mesh>,
    ),
    masks: &[(usize, opts_bevy::ground::CpuImage)],
    layer_total: usize,
) -> Option<()> {
    use opts_bevy::ground::CpuImage;
    use opts_bevy::ground_material::{layer_array, layer_array_size};
    // Decode the base and every painted layer once per map and upload them as one
    // mipmapped array.
    let dirs = vec![
        job.config.map_dir.clone(),
        job.config.map_dir.join("texture"),
    ];
    let mut wanted = vec![0];
    wanted.extend(painted_layers(&job.config.map_dir, layer_total));
    // A mask found through the install index but missed by the folder scan still counts.
    for (n, _) in masks {
        if !wanted.contains(n) {
            wanted.push(*n);
        }
    }
    wanted.sort_unstable();
    let decoded = decode_layers(job, (meshes, materials, images), (&wanted, &dirs))?;
    let largest = decoded
        .iter()
        .map(|(_, d, _)| d.width.max(d.height))
        .max()
        .unwrap_or(512);
    let size = layer_array_size(decoded.len(), largest);
    let refs: Vec<&CpuImage> = decoded.iter().map(|(_, d, _)| d).collect();
    let layers = images.add(layer_array(&refs, size));
    // Detail textures modulate their layer; a missing one is white (no change).
    let white = CpuImage {
        width: 1,
        height: 1,
        rgba: vec![255; 4],
    };
    let detail_refs: Vec<&CpuImage> = decoded
        .iter()
        .map(|(_, _, d)| d.as_ref().unwrap_or(&white))
        .collect();
    let details = images.add(layer_array(&detail_refs, 256));
    let mut index = vec![None; layer_total];
    for (slot, (n, _, _)) in decoded.iter().enumerate() {
        index[*n] = Some(slot as u32);
    }
    tracing::info!(
        listed = layer_total,
        in_array = decoded.len(),
        size,
        "ground layer array built"
    );
    job.ground_array = Some(GroundArray {
        layers,
        details,
        index,
    });
    Some(())
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
    use opts_bevy::ground_material::{ground_extension, mask_array, GroundSlot, MAX_TILE_LAYERS};
    let file = job.tile_files.get(&key)?.clone();
    let layer_total = job.manifest.ground_textures.len();
    let masks = read_masks(job, &file, layer_total);
    if masks.is_empty() {
        return None;
    }
    if job.ground_array.is_none() {
        build_ground_array(job, (images, materials, meshes), &masks, layer_total)?;
    }
    let array = job.ground_array.as_ref()?;
    let base = array.index.first().copied().flatten()?;
    let slot = |n: usize, layer: u32| {
        let params = &job.manifest.ground_textures[n].params;
        GroundSlot {
            layer,
            repeats: params[1].max(1.0),
            detail_repeats: params[2].max(1.0),
        }
    };
    let mut slots = vec![slot(0, base)];
    let mut slot_masks: Vec<Option<&CpuImage>> = vec![None];
    for (n, mask) in &masks {
        let Some(layer) = array.index.get(*n).copied().flatten() else {
            continue;
        };
        if slots.len() == MAX_TILE_LAYERS {
            tracing::warn!(tile = ?key, limit = MAX_TILE_LAYERS, "tile paints more ground layers than the shader blends; the highest are left out");
            break;
        }
        slots.push(slot(*n, layer));
        slot_masks.push(Some(mask));
    }
    let size = slot_masks
        .iter()
        .flatten()
        .map(|m| m.width.max(m.height))
        .max()
        .unwrap_or(512)
        .clamp(256, 1024);
    let mask_handle = images.add(mask_array(&slot_masks, size));
    Some(ground_materials.add(GroundMaterial {
        base: terrain_material(None),
        extension: ground_extension(
            array.layers.clone(),
            array.details.clone(),
            mask_handle,
            &slots,
        ),
    }))
}

#[cfg(test)]
mod tests;
