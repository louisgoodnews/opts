//! Texture format sniffing, DirectDraw Surface (`.dds`) decoding, and fallback image loaders.
//!
//! OMSI assets use `.dds`, `.bmp`, `.tga`, `.png`, and `.jpg` image files.
//! Notably, over 1,170 corpus files have file extensions that do not match their
//! underlying content (such as BMP files named with `.dds` extensions).
//! This module provides magic-byte content sniffing and decoder routing into native
//! Bevy [`Image`] assets.

#![deny(unsafe_code)]

use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::texture::Image;
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

    let extent = Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };

    // Determine texture format and prepare data buffer
    let (format, data) = parse_dds_format_and_data(&dds)?;

    let mut image = Image::default();
    image.texture_descriptor.size = extent;
    image.texture_descriptor.format = format;
    image.texture_descriptor.dimension = TextureDimension::D2;
    image.texture_descriptor.mip_level_count = dds.header.mip_map_count.unwrap_or(1).max(1);
    image.asset_usage = RenderAssetUsages::default();
    image.data = data;

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
    if bytes.is_empty() {
        return Err(TextureError::EmptyBuffer);
    }

    let dyn_img =
        image::load_from_memory(bytes).map_err(|e| TextureError::ImageDecode(e.to_string()))?;

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
        ImageFormat::Bmp | ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::Tga => {
            image_to_bevy_image(bytes)
        }
        ImageFormat::Unknown => {
            // Attempt generic image decode fallback before giving up
            image_to_bevy_image(bytes).map_err(|_| TextureError::UnknownFormat)
        }
    }
}

#[cfg(test)]
mod tests {
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
}
