//! Painted ground: layered tile materials.

use bevy::prelude::*;
use opts_bevy::ground_material::GroundMaterial;
use opts_bevy::{terrain_material, ContentLoader};

/// Asset stores the loading systems write to (grouped to stay within Bevy's parameter limit).
use super::WorldLoadJob;

mod discover;
use discover::{painted_layers, read_masks};
mod decode;
use decode::build_ground_array;

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
