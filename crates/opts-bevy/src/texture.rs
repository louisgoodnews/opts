//! Texture format sniffing, DirectDraw Surface (`.dds`) decoding, and fallback image loaders.
//!
//! OMSI assets use `.dds`, `.bmp`, `.tga`, `.png`, and `.jpg` image files.
//! Notably, over 1,170 corpus files have file extensions that do not match their
//! underlying content (such as BMP files named with `.dds` extensions).
//! This module provides magic-byte content sniffing and decoder routing into native
//! Bevy [`Image`] assets.

#![deny(unsafe_code)]

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use ddsfile::{D3DFormat, Dds, DxgiFormat, FourCC, PixelFormatFlags};
use thiserror::Error;

/// Recognized image container formats determined by content sniffing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageFormat {
    /// Microsoft DirectDraw Surface (`DDS ` magic).
    Dds,
    /// Windows Bitmap (`BM` magic).
    Bmp,
    /// Portable Network Graphics (`\x89PNG` magic).
    Png,
    /// JPEG image (`\xFF\xD8` magic).
    Jpeg,
    /// Truevision TGA image (identified by header heuristics).
    Tga,
    /// Unrecognized format.
    Unknown,
}

/// Errors occurring during texture format sniffing or image decoding.
#[derive(Debug, Error)]
pub enum TextureError {
    /// Buffer is empty.
    #[error("texture buffer is empty")]
    EmptyBuffer,

    /// Unrecognized or unsupported image container magic bytes.
    #[error("unknown or unsupported image container format")]
    UnknownFormat,

    /// Error decoding DirectDraw Surface (.dds).
    #[error("failed to decode DDS: {0}")]
    DdsDecode(String),

    /// Error decoding generic image format.
    #[error("failed to decode image: {0}")]
    ImageDecode(String),

    /// Texture format or channel layout is unsupported.
    #[error("unsupported texture format: {0}")]
    UnsupportedFormat(String),
}

/// Inspects the header magic bytes of an image buffer to detect its actual format.
///
/// This sniffing step is critical for OMSI assets, where over 900 BMP textures
/// are disguised with a `.dds` file extension.
#[must_use]
pub fn sniff_image_format(bytes: &[u8]) -> ImageFormat {
    if bytes.len() >= 4 && &bytes[0..4] == b"DDS " {
        ImageFormat::Dds
    } else if bytes.len() >= 2 && &bytes[0..2] == b"BM" {
        ImageFormat::Bmp
    } else if bytes.len() >= 8 && &bytes[0..8] == b"\x89PNG\r\n\x1a\n" {
        ImageFormat::Png
    } else if bytes.len() >= 2 && bytes[0..2] == [0xFF, 0xD8] {
        ImageFormat::Jpeg
    } else if is_plausible_tga(bytes) {
        ImageFormat::Tga
    } else {
        ImageFormat::Unknown
    }
}

/// Helper function to validate Truevision TGA header plausibility.
fn is_plausible_tga(bytes: &[u8]) -> bool {
    // TGA headers are 18 bytes:
    // byte 1: color_map_type (0 or 1)
    // byte 2: image_type (1, 2, 3, 9, 10, 11)
    // bytes 12..14: width (u16 little-endian > 0)
    // bytes 14..16: height (u16 little-endian > 0)
    // byte 16: pixel_depth (8, 15, 16, 24, 32)
    if bytes.len() < 18 {
        return false;
    }
    let color_map_type = bytes[1];
    let image_type = bytes[2];
    let width = u16::from_le_bytes([bytes[12], bytes[13]]);
    let height = u16::from_le_bytes([bytes[14], bytes[15]]);
    let pixel_depth = bytes[16];

    (color_map_type == 0 || color_map_type == 1)
        && matches!(image_type, 1 | 2 | 3 | 9 | 10 | 11)
        && width > 0
        && height > 0
        && matches!(pixel_depth, 8 | 15 | 16 | 24 | 32)
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
    let (width, height) = if matches!(
        format,
        TextureFormat::Bc1RgbaUnormSrgb
            | TextureFormat::Bc2RgbaUnormSrgb
            | TextureFormat::Bc3RgbaUnormSrgb
    ) {
        (width.div_ceil(4) * 4, height.div_ceil(4) * 4)
    } else {
        (width, height)
    };
    let extent = Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };

    let mut image = Image::default();
    image.texture_descriptor.size = extent;
    image.texture_descriptor.format = format;
    image.texture_descriptor.dimension = TextureDimension::D2;
    let mut mips = dds.header.mip_map_count.unwrap_or(1).max(1);
    let mut data = data;
    // The GPU upload slices the buffer by the mip sizes derived from the (possibly rounded)
    // extent; when that disagrees with the file's chain, keep only the base level.
    let (block_width, block_height, block_bytes) = match format {
        TextureFormat::Bc1RgbaUnormSrgb => (4, 4, 8),
        TextureFormat::Bc2RgbaUnormSrgb | TextureFormat::Bc3RgbaUnormSrgb => (4, 4, 16),
        TextureFormat::Rgba8UnormSrgb => (1, 1, 4),
        TextureFormat::R8Unorm => (1, 1, 1),
        _ => (1, 1, 4),
    };
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
        mips = 1;
    } else if data.len() > expected {
        data.truncate(expected);
    }
    image.texture_descriptor.mip_level_count = mips;
    image.asset_usage = RenderAssetUsages::default();
    image.data = Some(data);

    Ok(image)
}

/// Helper extracting texture format and pixel data from a parsed [`Dds`] file.
fn parse_dds_format_and_data(dds: &Dds) -> Result<(TextureFormat, Vec<u8>), TextureError> {
    let pf = &dds.header.spf;

    // 1. Check FourCC for block compression formats
    if pf.flags.contains(PixelFormatFlags::FOURCC) {
        if let Some(fourcc) = pf.fourcc.as_ref() {
            if fourcc.0 == FourCC::DXT1 {
                return Ok((TextureFormat::Bc1RgbaUnormSrgb, dds.data.clone()));
            } else if fourcc.0 == FourCC::DXT3 {
                return Ok((TextureFormat::Bc2RgbaUnormSrgb, dds.data.clone()));
            } else if fourcc.0 == FourCC::DXT5 {
                return Ok((TextureFormat::Bc3RgbaUnormSrgb, dds.data.clone()));
            }
        }
    }

    // 2. Check D3DFormat / DxgiFormat if available
    if let Some(d3d_fmt) = dds.get_d3d_format() {
        match d3d_fmt {
            D3DFormat::DXT1 => return Ok((TextureFormat::Bc1RgbaUnormSrgb, dds.data.clone())),
            D3DFormat::DXT3 => return Ok((TextureFormat::Bc2RgbaUnormSrgb, dds.data.clone())),
            D3DFormat::DXT5 => return Ok((TextureFormat::Bc3RgbaUnormSrgb, dds.data.clone())),
            D3DFormat::A8R8G8B8 => return convert_bgra_to_rgba(&dds.data),
            D3DFormat::A8B8G8R8 => return Ok((TextureFormat::Rgba8UnormSrgb, dds.data.clone())),
            D3DFormat::R8G8B8 => return convert_rgb24_to_rgba(&dds.data),
            D3DFormat::A8 => return Ok((TextureFormat::R8Unorm, dds.data.clone())),
            _ => {}
        }
    }

    if let Some(dxgi_fmt) = dds.get_dxgi_format() {
        match dxgi_fmt {
            DxgiFormat::BC1_UNorm | DxgiFormat::BC1_UNorm_sRGB => {
                return Ok((TextureFormat::Bc1RgbaUnormSrgb, dds.data.clone()));
            }
            DxgiFormat::BC2_UNorm | DxgiFormat::BC2_UNorm_sRGB => {
                return Ok((TextureFormat::Bc2RgbaUnormSrgb, dds.data.clone()));
            }
            DxgiFormat::BC3_UNorm | DxgiFormat::BC3_UNorm_sRGB => {
                return Ok((TextureFormat::Bc3RgbaUnormSrgb, dds.data.clone()));
            }
            DxgiFormat::R8_UNorm => {
                return Ok((TextureFormat::R8Unorm, dds.data.clone()));
            }
            DxgiFormat::R8G8B8A8_UNorm | DxgiFormat::R8G8B8A8_UNorm_sRGB => {
                return Ok((TextureFormat::Rgba8UnormSrgb, dds.data.clone()));
            }
            _ => {}
        }
    }

    // 3. Fallback check for uncompressed RGB/RGBA/A8 by bit counts and masks
    let bit_count = pf.rgb_bit_count.unwrap_or(0);
    let r_mask = pf.r_bit_mask.unwrap_or(0);
    let g_mask = pf.g_bit_mask.unwrap_or(0);
    let b_mask = pf.b_bit_mask.unwrap_or(0);

    if pf.flags.contains(PixelFormatFlags::ALPHA) || (bit_count == 8 && pf.flags.is_empty()) {
        // 8-bit single channel alpha mask (used extensively in OMSI map tiles)
        Ok((TextureFormat::R8Unorm, dds.data.clone()))
    } else if bit_count == 32 {
        if r_mask == 0x00FF_0000 && g_mask == 0x0000_FF00 && b_mask == 0x0000_00FF {
            convert_bgra_to_rgba(&dds.data)
        } else {
            Ok((TextureFormat::Rgba8UnormSrgb, dds.data.clone()))
        }
    } else if bit_count == 24 {
        convert_rgb24_to_rgba(&dds.data)
    } else {
        Err(TextureError::UnsupportedFormat(format!(
            "unsupported DDS pixel format with {bit_count} bpp"
        )))
    }
}

/// Converts BGRA 32-bit pixel data to RGBA by swapping red and blue channels.
fn convert_bgra_to_rgba(data: &[u8]) -> Result<(TextureFormat, Vec<u8>), TextureError> {
    let mut rgba = data.to_vec();
    for chunk in rgba.chunks_exact_mut(4) {
        chunk.swap(0, 2);
    }
    Ok((TextureFormat::Rgba8UnormSrgb, rgba))
}

/// Converts 24-bit RGB/BGR pixel data to 32-bit RGBA by adding an opaque alpha channel.
fn convert_rgb24_to_rgba(data: &[u8]) -> Result<(TextureFormat, Vec<u8>), TextureError> {
    let pixel_count = data.len() / 3;
    let mut rgba = Vec::with_capacity(pixel_count * 4);
    for chunk in data.chunks_exact(3) {
        rgba.push(chunk[0]);
        rgba.push(chunk[1]);
        rgba.push(chunk[2]);
        rgba.push(255);
    }
    Ok((TextureFormat::Rgba8UnormSrgb, rgba))
}

/// Decodes standard image formats (`.bmp`, `.tga`, `.png`, `.jpg`) into a Bevy [`Image`].
///
/// Converts pixel data to RGBA8 format.
///
/// # Errors
/// Returns [`TextureError::ImageDecode`] if the image decoder fails.
pub fn image_to_bevy_image(bytes: &[u8]) -> Result<Image, TextureError> {
    decode_with(bytes, None)
}

/// Some tools write 24-bit TGAs whose descriptor claims 8 alpha bits; the `image` crate rejects
/// that combination. The descriptor's alpha-bit count is cleared for 24-bit (and 15/16-bit
/// without alpha) images.
fn repair_tga_header(bytes: &[u8]) -> std::borrow::Cow<'_, [u8]> {
    if bytes.len() > 18 && bytes[16] == 24 && bytes[17] & 0x0F != 0 {
        let mut fixed = bytes.to_vec();
        fixed[17] &= 0xF0;
        std::borrow::Cow::Owned(fixed)
    } else {
        std::borrow::Cow::Borrowed(bytes)
    }
}

/// Decodes with an explicit container format (TGA has no magic number, so it cannot be guessed).
fn decode_with(bytes: &[u8], format: Option<image::ImageFormat>) -> Result<Image, TextureError> {
    if bytes.is_empty() {
        return Err(TextureError::EmptyBuffer);
    }

    let repaired;
    let bytes = if format == Some(image::ImageFormat::Tga) {
        repaired = repair_tga_header(bytes);
        repaired.as_ref()
    } else {
        bytes
    };
    let decoded = match format {
        Some(f) => image::load_from_memory_with_format(bytes, f),
        None => image::load_from_memory(bytes),
    };
    let dyn_img = decoded.map_err(|e| TextureError::ImageDecode(e.to_string()))?;

    let width = dyn_img.width();
    let height = dyn_img.height();
    if width == 0 || height == 0 {
        return Err(TextureError::ImageDecode("zero width or height".into()));
    }

    let rgba = dyn_img.to_rgba8();
    let data = rgba.into_raw();

    let extent = Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };

    Ok(Image::new(
        extent,
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ))
}

/// Unified entry point: sniffs container format and loads any supported image into a Bevy [`Image`].
///
/// Correctly routes `.dds` files that secretly contain BMP headers.
///
/// # Errors
/// Returns [`TextureError`] if sniffing finds an unknown format or decoding fails.
pub fn load_bevy_image(bytes: &[u8]) -> Result<Image, TextureError> {
    if bytes.is_empty() {
        return Err(TextureError::EmptyBuffer);
    }

    let format = sniff_image_format(bytes);
    match format {
        ImageFormat::Dds => dds_to_bevy_image(bytes),
        ImageFormat::Tga => decode_with(bytes, Some(image::ImageFormat::Tga)),
        ImageFormat::Bmp | ImageFormat::Png | ImageFormat::Jpeg => image_to_bevy_image(bytes),
        ImageFormat::Unknown => {
            // Attempt generic image decode fallback before giving up
            image_to_bevy_image(bytes)
                .or_else(|_| decode_with(bytes, Some(image::ImageFormat::Tga)))
                .map_err(|_| TextureError::UnknownFormat)
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn bc3_block_decodes_alpha_and_colour() {
        // Alpha endpoints 0 and 0 (every texel transparent); colour endpoints white/black,
        // all indices 0 (= white).
        let mut block = vec![0u8; 8];
        block.extend_from_slice(&[0xFF, 0xFF, 0x00, 0x00, 0, 0, 0, 0]);
        let mut img = Image::new(
            Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            block,
            TextureFormat::Bc3RgbaUnormSrgb,
            RenderAssetUsages::default(),
        );
        img.texture_descriptor.mip_level_count = 1;
        let out = decode_to_rgba8(&img).expect("decodes");
        let px = out.data.as_ref().unwrap();
        assert_eq!(&px[0..3], &[255, 255, 255]);
        assert!(px.chunks_exact(4).all(|p| p[3] < 255));
    }

    use super::*;

    #[test]
    fn test_sniff_magic_bytes() {
        assert_eq!(sniff_image_format(b"DDS \x00\x00"), ImageFormat::Dds);
        assert_eq!(sniff_image_format(b"BM\x00\x00"), ImageFormat::Bmp);
        assert_eq!(
            sniff_image_format(b"\x89PNG\r\n\x1a\n\x00\x00"),
            ImageFormat::Png
        );
        assert_eq!(
            sniff_image_format(&[0xFF, 0xD8, 0xFF, 0xE0]),
            ImageFormat::Jpeg
        );
        assert_eq!(sniff_image_format(b"RIFF\x00\x00"), ImageFormat::Unknown);
        assert_eq!(sniff_image_format(b""), ImageFormat::Unknown);
    }

    #[test]
    fn test_empty_buffer_returns_error() {
        assert!(matches!(
            load_bevy_image(&[]),
            Err(TextureError::EmptyBuffer)
        ));
    }

    /// Builds a DXT5 DDS whose mip chain follows `height` exactly like the files in the wild.
    fn dxt5_dds(width: u32, height: u32, mips: u32) -> Vec<u8> {
        let mut b = Vec::new();
        let mut put = |v: u32| b.extend_from_slice(&v.to_le_bytes());
        put(0x2053_4444); // "DDS "
        put(124);
        put(0x0002_1007 | 0x0008_0000);
        put(height);
        put(width);
        put(0);
        put(0);
        put(mips);
        for _ in 0..11 {
            put(0);
        }
        put(32);
        put(4); // DDPF_FOURCC
        put(u32::from_le_bytes(*b"DXT5"));
        for _ in 0..5 {
            put(0);
        }
        put(0x1000);
        for _ in 0..4 {
            put(0);
        }
        let (mut w, mut h) = (width, height);
        for _ in 0..mips {
            let n = (w.div_ceil(4).max(1) * h.div_ceil(4).max(1) * 16) as usize;
            b.extend(std::iter::repeat_n(0u8, n));
            w = (w / 2).max(1);
            h = (h / 2).max(1);
        }
        b
    }

    #[test]
    fn block_compressed_extent_and_mips_stay_consistent() {
        let img = dds_to_bevy_image(&dxt5_dds(64, 533, 7)).expect("decodes");
        assert_eq!(img.texture_descriptor.size.height, 536);
        assert_eq!(img.texture_descriptor.mip_level_count, 7);
        // A header that claims more levels than the data holds falls back to the base level.
        let mut bytes = dxt5_dds(64, 64, 3);
        bytes[28..32].copy_from_slice(&7u32.to_le_bytes());
        let img = dds_to_bevy_image(&bytes).expect("decodes");
        assert_eq!(img.texture_descriptor.mip_level_count, 1);
        assert_eq!(img.data.as_ref().unwrap().len(), 16 * 16 * 16);
    }

    #[test]
    fn test_non_block_aligned_mip_chain_fallback() {
        // Dimensions like hochspannungsmast.dds (256x1025, 11 mips).
        // Rounding 1025 to 1028 shifts intermediate mip sizes such that wgpu expected size (351376)
        // exceeds the file data size (350608). The loader must fall back to base level (mips = 1)
        // and truncate data to 263168, preventing wgpu slice out-of-range panics.
        let bytes = dxt5_dds(256, 1025, 11);
        assert_eq!(bytes.len(), 128 + 350608);
        let img = dds_to_bevy_image(&bytes).expect("decodes without panic");
        assert_eq!(img.texture_descriptor.size.width, 256);
        assert_eq!(img.texture_descriptor.size.height, 1028);
        assert_eq!(img.texture_descriptor.mip_level_count, 1);
        assert_eq!(img.data.as_ref().unwrap().len(), 263168);
    }

    #[test]
    fn test_dds_with_trailing_bytes_truncated() {
        let mut bytes = dxt5_dds(64, 64, 3);
        bytes.extend_from_slice(&[0xCC; 256]); // 256 trailing bytes
        let img = dds_to_bevy_image(&bytes).expect("decodes");
        assert_eq!(img.texture_descriptor.mip_level_count, 3);
        assert_eq!(
            img.data.as_ref().unwrap().len(),
            (16 * 16 + 8 * 8 + 4 * 4) * 16
        );
    }
}

#[cfg(test)]
mod tga_repair_tests {
    use super::*;

    #[test]
    fn a_24_bit_tga_claiming_alpha_bits_decodes() {
        // 2x1 uncompressed true-colour TGA, 24 bpp, descriptor 0x08 (as written by some tools).
        let mut tga = vec![0u8; 18];
        tga[2] = 2;
        tga[12] = 2;
        tga[14] = 1;
        tga[16] = 24;
        tga[17] = 0x08;
        tga.extend_from_slice(&[0, 0, 255, 0, 255, 0]);
        let img = decode_with(&tga, Some(image::ImageFormat::Tga)).expect("decodes");
        assert_eq!((img.width(), img.height()), (2, 1));
    }
}

/// Decodes a block-compressed (BC1/BC2/BC3) or plain RGBA8 image to RGBA8 pixels.
///
/// Used where texel values are needed on the CPU (transparency maps composed into a paint
/// texture). Returns `None` for other formats or truncated data.
#[must_use]
pub fn decode_to_rgba8(image: &Image) -> Option<Image> {
    use bevy::render::render_resource::TextureFormat as F;
    let (w, h) = (image.width() as usize, image.height() as usize);
    let data = image.data.as_deref()?;
    let format = image.texture_descriptor.format;
    let (block_bytes, kind) = match format {
        F::Rgba8Unorm | F::Rgba8UnormSrgb => {
            return (data.len() == w * h * 4).then(|| image.clone());
        }
        F::Bc1RgbaUnorm | F::Bc1RgbaUnormSrgb => (8, 1),
        F::Bc2RgbaUnorm | F::Bc2RgbaUnormSrgb => (16, 2),
        F::Bc3RgbaUnorm | F::Bc3RgbaUnormSrgb => (16, 3),
        _ => return None,
    };
    let (bw, bh) = (w.div_ceil(4), h.div_ceil(4));
    if data.len() < bw * bh * block_bytes {
        return None;
    }
    let mut out = vec![0u8; w * h * 4];
    for by in 0..bh {
        for bx in 0..bw {
            let block = &data[(by * bw + bx) * block_bytes..][..block_bytes];
            let colour = &block[block_bytes - 8..];
            let mut alpha = [255u8; 16];
            match kind {
                2 => {
                    for i in 0..16 {
                        let nib = (block[i / 2] >> (4 * (i % 2))) & 15;
                        alpha[i] = nib * 17;
                    }
                }
                3 => {
                    let (a0, a1) = (u16::from(block[0]), u16::from(block[1]));
                    let mut table = [0u16; 8];
                    table[0] = a0;
                    table[1] = a1;
                    if a0 > a1 {
                        for k in 0..6u16 {
                            table[2 + k as usize] = ((6 - k) * a0 + (k + 1) * a1) / 7;
                        }
                    } else {
                        for k in 0..4u16 {
                            table[2 + k as usize] = ((4 - k) * a0 + (k + 1) * a1) / 5;
                        }
                        table[7] = 255;
                    }
                    let mut bits = 0u64;
                    for (i, b) in block[2..8].iter().enumerate() {
                        bits |= u64::from(*b) << (8 * i);
                    }
                    for (i, a) in alpha.iter_mut().enumerate() {
                        *a = table[((bits >> (3 * i)) & 7) as usize] as u8;
                    }
                }
                _ => {}
            }
            let c0 = u16::from_le_bytes([colour[0], colour[1]]);
            let c1 = u16::from_le_bytes([colour[2], colour[3]]);
            let expand = |c: u16| {
                let (r, g, b) = (
                    u32::from((c >> 11) & 31),
                    u32::from((c >> 5) & 63),
                    u32::from(c & 31),
                );
                [
                    (r * 255 / 31) as u32,
                    (g * 255 / 63) as u32,
                    (b * 255 / 31) as u32,
                ]
            };
            let (p0, p1) = (expand(c0), expand(c1));
            let mut palette = [[0u32; 4]; 4];
            palette[0] = [p0[0], p0[1], p0[2], 255];
            palette[1] = [p1[0], p1[1], p1[2], 255];
            if c0 > c1 || kind != 1 {
                for k in 0..3 {
                    palette[2][k] = (2 * p0[k] + p1[k]) / 3;
                    palette[3][k] = (p0[k] + 2 * p1[k]) / 3;
                }
                palette[2][3] = 255;
                palette[3][3] = 255;
            } else {
                for k in 0..3 {
                    palette[2][k] = (p0[k] + p1[k]) / 2;
                }
                palette[2][3] = 255;
                palette[3] = [0, 0, 0, 0];
            }
            let idx = u32::from_le_bytes([colour[4], colour[5], colour[6], colour[7]]);
            for i in 0..16 {
                let (x, y) = (bx * 4 + i % 4, by * 4 + i / 4);
                if x >= w || y >= h {
                    continue;
                }
                let c = palette[((idx >> (2 * i)) & 3) as usize];
                let o = (y * w + x) * 4;
                out[o] = c[0] as u8;
                out[o + 1] = c[1] as u8;
                out[o + 2] = c[2] as u8;
                out[o + 3] = if kind == 1 { c[3] as u8 } else { alpha[i] };
            }
        }
    }
    let mut result = image.clone();
    result.data = Some(out);
    result.texture_descriptor.format = if format.is_srgb() {
        F::Rgba8UnormSrgb
    } else {
        F::Rgba8Unorm
    };
    result.texture_descriptor.mip_level_count = 1;
    Some(result)
}
