//! Layered ground material: a standard PBR material whose base colour is blended on the GPU from
//! the map's `[groundtex]` layers and the tile's per-layer masks.

use bevy::asset::{load_internal_asset, Asset};
use bevy::pbr::{ExtendedMaterial, MaterialExtension, MaterialPlugin, StandardMaterial};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, ShaderRef, TextureDimension, TextureFormat, TextureViewDescriptor,
    TextureViewDimension,
};
use bevy::render::texture::{
    ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor,
};

use crate::ground::CpuImage;

/// Most layers a tile can blend (the shader packs repeat counts into two `vec4`s).
pub const MAX_GROUND_LAYERS: usize = 8;

const SHADER: Handle<Shader> = Handle::weak_from_u128(0x6f70_7473_6772_6f75_6e64_6c61_7965_7273);

/// Bind-group extension of [`StandardMaterial`] carrying the layer and mask arrays.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct GroundExtension {
    /// Layer textures (shared by every tile of a map), with mip chains.
    #[texture(100, dimension = "2d_array")]
    #[sampler(101)]
    pub layers: Handle<Image>,
    /// Per-tile blend masks (R8), one array layer per ground layer.
    #[texture(102, dimension = "2d_array")]
    #[sampler(103)]
    pub masks: Handle<Image>,
    /// Repeats per tile edge of layers 0–3.
    #[uniform(104)]
    pub repeats_a: Vec4,
    /// Repeats per tile edge of layers 4–7.
    #[uniform(105)]
    pub repeats_b: Vec4,
    /// Number of layers in use (x component).
    #[uniform(106)]
    pub layer_count: UVec4,
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
        load_internal_asset!(app, SHADER, "shaders/ground_layers.wgsl", Shader::from_wgsl);
        app.add_plugins(MaterialPlugin::<GroundMaterial>::default());
    }
}

/// Bilinearly resamples `img` to `size × size`.
fn resample(img: &CpuImage, size: usize) -> Vec<[u8; 4]> {
    let mut out = Vec::with_capacity(size * size);
    for y in 0..size {
        for x in 0..size {
            let p = img.sample_wrap(
                (x as f32 + 0.5) / size as f32,
                (y as f32 + 0.5) / size as f32,
            );
            out.push([p[0] as u8, p[1] as u8, p[2] as u8, p[3] as u8]);
        }
    }
    out
}

/// Box-filters an RGBA8 `size × size` image to half size.
fn halve(src: &[[u8; 4]], size: usize) -> Vec<[u8; 4]> {
    let half = (size / 2).max(1);
    let mut out = Vec::with_capacity(half * half);
    for y in 0..half {
        for x in 0..half {
            let mut acc = [0u32; 4];
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let p = src[(y * 2 + dy).min(size - 1) * size + (x * 2 + dx).min(size - 1)];
                for k in 0..4 {
                    acc[k] += u32::from(p[k]);
                }
            }
            out.push(acc.map(|v| ((v + 2) / 4) as u8));
        }
    }
    out
}

/// Stacks `layers` (resampled to `size`, a power of two) into a mipmapped sRGB 2D-array texture.
#[must_use]
pub fn layer_array(layers: &[&CpuImage], size: usize) -> Image {
    let mips = size.ilog2() as usize + 1;
    let mut data = Vec::new();
    for layer in layers {
        let mut level = resample(layer, size);
        let mut dim = size;
        for m in 0..mips {
            data.extend(level.iter().flatten());
            if m + 1 < mips {
                level = halve(&level, dim);
                dim = (dim / 2).max(1);
            }
        }
    }
    let mut image = Image::new_fill(
        Extent3d {
            width: size as u32,
            height: size as u32,
            depth_or_array_layers: layers.len() as u32,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.mip_level_count = mips as u32;
    image.data = data;
    image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::D2Array),
        ..default()
    });
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: 8,
        ..default()
    });
    image
}

/// Stacks blend masks (`None` = fully off, except layer 0 which is fully on) into an R8 array.
#[must_use]
pub fn mask_array(masks: &[Option<&CpuImage>], size: usize) -> Image {
    let mut data = Vec::with_capacity(masks.len() * size * size);
    for (i, mask) in masks.iter().enumerate() {
        match mask {
            Some(m) => data.extend(resample(m, size).iter().map(|p| p[3])),
            None => data.extend(std::iter::repeat_n(
                if i == 0 { 255 } else { 0 },
                size * size,
            )),
        }
    }
    let mut image = Image::new_fill(
        Extent3d {
            width: size as u32,
            height: size as u32,
            depth_or_array_layers: masks.len() as u32,
        },
        TextureDimension::D2,
        &[0],
        TextureFormat::R8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.data = data;
    image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::D2Array),
        ..default()
    });
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::ClampToEdge,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        ..default()
    });
    image
}

/// Builds the extension for one tile.
#[must_use]
pub fn ground_extension(
    layers: Handle<Image>,
    masks: Handle<Image>,
    repeats: &[f32],
) -> GroundExtension {
    let r = |i: usize| repeats.get(i).copied().unwrap_or(1.0);
    GroundExtension {
        layers,
        masks,
        repeats_a: Vec4::new(r(0), r(1), r(2), r(3)),
        repeats_b: Vec4::new(r(4), r(5), r(6), r(7)),
        layer_count: UVec4::new(repeats.len().min(MAX_GROUND_LAYERS) as u32, 0, 0, 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(v: u8) -> CpuImage {
        CpuImage {
            width: 4,
            height: 4,
            rgba: [v, v, v, 255].repeat(16),
        }
    }

    #[test]
    fn layer_array_holds_a_full_mip_chain_per_layer() {
        let (a, b) = (solid(10), solid(200));
        let img = layer_array(&[&a, &b], 8);
        // 8x8 + 4x4 + 2x2 + 1x1 texels per layer, RGBA8.
        let per_layer = (64 + 16 + 4 + 1) * 4;
        assert_eq!(img.data.len(), 2 * per_layer);
        assert_eq!(img.texture_descriptor.mip_level_count, 4);
        assert_eq!(
            img.data[per_layer], 200,
            "second layer starts after the first layer's mips"
        );
    }

    #[test]
    fn mask_array_defaults_base_on_and_missing_layers_off() {
        let m = CpuImage {
            width: 2,
            height: 2,
            rgba: [0, 0, 0, 128].repeat(4),
        };
        let img = mask_array(&[None, Some(&m), None], 2);
        assert_eq!(&img.data[0..4], &[255; 4]);
        assert_eq!(&img.data[4..8], &[128; 4]);
        assert_eq!(&img.data[8..12], &[0; 4]);
    }
}
