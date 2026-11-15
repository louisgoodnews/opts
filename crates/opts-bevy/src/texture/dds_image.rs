//! DirectDraw Surface decoding into a Bevy image: mip chains, the rounded extent and the upload layout.

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use ddsfile::Dds;

use super::dds_chain;
use super::error::TextureError;
use super::pixel::parse_dds_format_and_data;

static DROPPED_DDS_CHAINS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Number of DDS files decoded in this process whose declared mipmap chain disagreed with the
/// data and was reduced to the base level (Q-TQ03). Uncompressed ones get a generated chain
/// afterwards; block-compressed ones stay without.
#[must_use]
pub fn dropped_dds_chains() -> u64 {
    DROPPED_DDS_CHAINS.load(std::sync::atomic::Ordering::Relaxed)
}

/// Block-compressed textures must have dimensions that are multiples of the 4×4 block size
/// (wgpu rejects the texture otherwise and the validation error is fatal). The data already
/// holds whole blocks, so rounding the extent up keeps the layout valid at the cost of a
/// sub-percent UV scale (e.g. 533 → 536 rows).
pub(super) fn rounded_extent(format: TextureFormat, width: u32, height: u32) -> (u32, u32) {
    if matches!(
        format,
        TextureFormat::Bc1RgbaUnormSrgb
            | TextureFormat::Bc2RgbaUnormSrgb
            | TextureFormat::Bc3RgbaUnormSrgb
    ) {
        (width.div_ceil(4) * 4, height.div_ceil(4) * 4)
    } else {
        (width, height)
    }
}

/// Makes the data match the mip chain the GPU upload expects for the (rounded) extent: re-lays out
/// a block-compressed chain stored for the file's own size, pads or cuts to the expected length,
/// and keeps only the base level when the chain does not fit. Returns the data and the level
/// count.
fn fit_mip_chain(
    mut data: Vec<u8>,
    format: TextureFormat,
    file_size: (u32, u32),
    ((width, height), mut mips): ((u32, u32), u32),
) -> Result<(Vec<u8>, u32), TextureError> {
    // The GPU upload slices the buffer by the mip sizes derived from the (possibly rounded)
    // extent; when that disagrees with the file's chain, keep only the base level.
    let (block_width, block_height, block_bytes) = match format {
        TextureFormat::Bc1RgbaUnormSrgb => (4, 4, 8),
        TextureFormat::Bc2RgbaUnormSrgb | TextureFormat::Bc3RgbaUnormSrgb => (4, 4, 16),
        TextureFormat::Rgba8UnormSrgb => (1, 1, 4),
        TextureFormat::R8Unorm => (1, 1, 1),
        _ => (1, 1, 4),
    };
    // Block-compressed files with a size that is not a multiple of 4 store their chain for the
    // file's own size; re-lay it out for the rounded extent the GPU expects.
    if mips > 1 && block_width == 4 && file_size != (width, height) {
        if let Some(chain) =
            dds_chain::relayout_chain(&data, file_size, (width, height), mips, block_bytes)
        {
            data = chain;
        }
    }
    let level_bytes = |w: u32, h: u32| {
        (w.div_ceil(block_width).max(1) as usize)
            * (h.div_ceil(block_height).max(1) as usize)
            * block_bytes
    };
    let mut expected = 0usize;
    let (mut w, mut h) = (width, height);
    for _ in 0..mips {
        expected += level_bytes(w, h);
        w = (w / 2).max(1);
        h = (h / 2).max(1);
    }
    if data.len() < expected {
        let base = level_bytes(width, height);
        if data.len() < base {
            if data.len() + 1024 >= base {
                data.resize(base, 0);
            } else {
                return Err(TextureError::DdsDecode(
                    "DDS data shorter than its base level".into(),
                ));
            }
        } else {
            data.truncate(base);
        }
        if mips > 1 {
            DROPPED_DDS_CHAINS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        mips = 1;
    } else if data.len() > expected {
        data.truncate(expected);
    }
    Ok((data, mips))
}

/// Decodes a DirectDraw Surface (`.dds`) buffer into a Bevy [`Image`].
///
/// Supports DXT1 (BC1), DXT3 (BC2), DXT5 (BC3), uncompressed 32-bit RGBA/BGRA,
/// 24-bit RGB, and 8-bit A8 alpha mask textures.
///
/// # Errors
/// Returns [`TextureError::DdsDecode`] or [`TextureError::UnsupportedFormat`] if
/// the header or pixel format is invalid.
pub fn dds_to_bevy_image(bytes: &[u8]) -> Result<Image, TextureError> {
    if bytes.is_empty() {
        return Err(TextureError::EmptyBuffer);
    }

    let mut cursor = std::io::Cursor::new(bytes);
    let dds = Dds::read(&mut cursor).map_err(|e| TextureError::DdsDecode(e.to_string()))?;

    let width = dds.header.width;
    let height = dds.header.height;
    if width == 0 || height == 0 {
        return Err(TextureError::DdsDecode("zero width or height".into()));
    }

    // Determine texture format and prepare data buffer
    let (format, data) = parse_dds_format_and_data(&dds)?;

    // Block-compressed textures must have dimensions that are multiples of the 4×4 block size
    // (wgpu rejects the texture otherwise and the validation error is fatal). The data already
    // holds whole blocks, so rounding the extent up keeps the layout valid at the cost of a
    // sub-percent UV scale (e.g. 533 → 536 rows).
    let (width, height) = rounded_extent(format, width, height);
    let extent = Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };

    let mut image = Image::default();
    image.texture_descriptor.size = extent;
    image.texture_descriptor.format = format;
    image.texture_descriptor.dimension = TextureDimension::D2;
    let mips = dds.header.mip_map_count.unwrap_or(1).max(1);
    let (data, mips) = fit_mip_chain(
        data,
        format,
        (dds.header.width, dds.header.height),
        ((width, height), mips),
    )?;
    image.texture_descriptor.mip_level_count = mips;
    image.asset_usage = RenderAssetUsages::default();
    image.data = Some(data);

    Ok(image)
}
