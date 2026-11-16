//! Layered ground material: a standard PBR material whose base colour is blended on the GPU from
//! the map's `[groundtex]` layers and the tile's per-layer masks.
//!
//! A map can list many layers (55 in the installed corpus) while one tile paints few of them (at
//! most 18 plus the base). The map's layers live in one array texture; each tile names the layers
//! it paints in **slots** ([`GroundSlot`]): slot 0 is the base, slot `k` blends array layer
//! `slots[k].layer` by mask `k`.

use bevy::asset::RenderAssetUsages;
use bevy::asset::{load_internal_asset, Asset};
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::pbr::{ExtendedMaterial, MaterialExtension, MaterialPlugin, StandardMaterial};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor,
    TextureViewDimension,
};
use bevy::shader::ShaderRef;

use crate::ground::CpuImage;

/// Most layers one tile can blend, the base included (the shader packs the slots into five
/// `vec4`s). The corpus maximum is 19 (Grand Paris-Moulon: 18 painted layers and the base).
pub const MAX_TILE_LAYERS: usize = 20;

/// Upper bound for the map's layer array in bytes (all mip levels): with many layers the common
/// layer size is halved until the array fits (see [`layer_array_size`]).
pub const GROUND_ARRAY_BUDGET_BYTES: usize = 256 << 20;

/// One blended layer of a tile.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroundSlot {
    /// Index into the map's layer (and detail) array.
    pub layer: u32,
    /// Repeats of the layer texture per tile edge (`[groundtex]` parameter 2).
    pub repeats: f32,
    /// Repeats of the detail texture per tile edge (`[groundtex]` parameter 3).
    pub detail_repeats: f32,
}

/// Common size (a power of two, 256–1024) of a layer array holding `count` layers whose largest
/// texture edge is `largest`: the largest edge rounded up, halved while the array with its mip
/// chains would exceed [`GROUND_ARRAY_BUDGET_BYTES`].
#[must_use]
pub fn layer_array_size(count: usize, largest: usize) -> usize {
    let mut size = largest.clamp(256, 1024).next_power_of_two();
    while size > 256 && count * size * size * 4 * 4 / 3 > GROUND_ARRAY_BUDGET_BYTES {
        size /= 2;
    }
    size
}

const SHADER: Handle<Shader> = bevy::asset::uuid_handle!("6f707473-6772-6f75-6e64-6c6179657273");

/// Bind-group extension of [`StandardMaterial`] carrying the layer and mask arrays.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct GroundExtension {
    /// Layer textures (shared by every tile of a map), with mip chains.
    #[texture(100, dimension = "2d_array")]
    #[sampler(101)]
    pub layers: Handle<Image>,
    /// Per-tile blend masks (R8), one array layer per slot (slot 0, the base, fully on).
    #[texture(102, dimension = "2d_array")]
    #[sampler(103)]
    pub masks: Handle<Image>,
    /// Layer-array index of each slot, four slots per `UVec4`.
    #[uniform(104)]
    pub slot_layers: [UVec4; MAX_TILE_LAYERS / 4],
    /// Repeats per tile edge of each slot's layer, four slots per `Vec4`.
    #[uniform(105)]
    pub repeats: [Vec4; MAX_TILE_LAYERS / 4],
    /// Number of slots in use (x component).
    #[uniform(106)]
    pub layer_count: UVec4,
    /// Detail textures, one per layer (white where a layer has none); they modulate the layer.
    #[texture(107, dimension = "2d_array")]
    #[sampler(108)]
    pub details: Handle<Image>,
    /// Detail repeats per tile edge of each slot, four slots per `Vec4`.
    #[uniform(109)]
    pub detail_repeats: [Vec4; MAX_TILE_LAYERS / 4],
    /// Wet-surface parameters: x = wetness 0..1, y = puddle coverage 0..1, z = snow cover 0..1
    /// (written by the weather, `in_game::weather::surfaces`). All zero (the default) leaves the
    /// shader output unchanged.
    #[uniform(111)]
    pub wet: Vec4,
}

impl MaterialExtension for GroundExtension {
    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }
    fn deferred_fragment_shader() -> ShaderRef {
        SHADER.into()
    }
}

/// The layered ground material.
pub type GroundMaterial = ExtendedMaterial<StandardMaterial, GroundExtension>;

/// Registers the shader and material type.
#[derive(Default)]
pub struct GroundMaterialPlugin;

impl Plugin for GroundMaterialPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, SHADER, "shaders/ground_layers.wesl", Shader::from_wesl);
        app.add_plugins(MaterialPlugin::<GroundMaterial>::default());
    }
}

mod array;

pub use array::*;


/// Builds the extension for one tile from its slots (slot 0 is the base; at most
/// [`MAX_TILE_LAYERS`] are used, `masks` holds one array layer per slot).
#[must_use]
pub fn ground_extension(
    layers: Handle<Image>,
    details: Handle<Image>,
    masks: Handle<Image>,
    slots: &[GroundSlot],
) -> GroundExtension {
    let used = slots.len().min(MAX_TILE_LAYERS);
    let mut slot_layers = [UVec4::ZERO; MAX_TILE_LAYERS / 4];
    let mut repeats = [Vec4::ONE; MAX_TILE_LAYERS / 4];
    let mut detail_repeats = [Vec4::ONE; MAX_TILE_LAYERS / 4];
    for (k, slot) in slots.iter().take(used).enumerate() {
        slot_layers[k / 4][k % 4] = slot.layer;
        repeats[k / 4][k % 4] = slot.repeats;
        detail_repeats[k / 4][k % 4] = slot.detail_repeats;
    }
    GroundExtension {
        layers,
        masks,
        slot_layers,
        repeats,
        layer_count: UVec4::new(used as u32, 0, 0, 0),
        details,
        detail_repeats,
        wet: Vec4::ZERO,
    }
}

#[cfg(test)]
mod tests;
