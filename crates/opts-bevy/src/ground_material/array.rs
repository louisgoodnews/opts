//! Building the ground texture arrays: the layer and mask arrays resampled to one size.

use super::*;

/// Bilinearly resamples `img` to `size × size`.
pub(super) fn resample(img: &CpuImage, size: usize) -> Vec<[u8; 4]> {
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
pub(super) fn halve(src: &[[u8; 4]], size: usize) -> Vec<[u8; 4]> {
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
    image.data = Some(data);
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

/// Stacks blend masks (`None` = fully off, except slot 0 which is fully on) into an R8 array.
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
    image.data = Some(data);
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
