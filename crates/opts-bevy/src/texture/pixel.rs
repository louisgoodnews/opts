//! Pixel formats of a parsed DDS file and the conversions to the RGBA8 layout of the GPU upload.

use bevy::render::render_resource::TextureFormat;
use ddsfile::{D3DFormat, Dds, DxgiFormat, FourCC, PixelFormatFlags};

use super::error::TextureError;

/// Helper extracting texture format and pixel data from a parsed [`Dds`] file.
pub(super) fn parse_dds_format_and_data(dds: &Dds) -> Result<(TextureFormat, Vec<u8>), TextureError> {
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
    } else if bit_count == 16 {
        // R5G6B5, A1R5G5B5, A4R4G4B4: Köln's building textures and many older OMSI objects.
        let a_mask = pf.a_bit_mask.unwrap_or(0);
        Ok(convert_rgb16_to_rgba(
            &dds.data,
            [r_mask, g_mask, b_mask, a_mask],
        ))
    } else {
        Err(TextureError::UnsupportedFormat(format!(
            "unsupported DDS pixel format with {bit_count} bpp"
        )))
    }
}

/// Expands 16-bit pixels with arbitrary channel masks (`[r, g, b, a]`) to RGBA8; a missing alpha
/// mask means opaque. A mask of 0 for a colour channel reads as 0.
pub(super) fn convert_rgb16_to_rgba(data: &[u8], masks: [u32; 4]) -> (TextureFormat, Vec<u8>) {
    // (shift, max) of a contiguous mask.
    let field = |m: u32| {
        if m == 0 {
            (0, 0)
        } else {
            let shift = m.trailing_zeros();
            (shift, m >> shift)
        }
    };
    let [fr, fg, fb, fa] = masks.map(field);
    let expand = |px: u32, f: (u32, u32), default: u8| {
        ((((px >> f.0) & f.1) * 255).checked_div(f.1)).map_or(default, |v| v as u8)
    };
    let mut rgba = Vec::with_capacity(data.len() * 2);
    for c in data.chunks_exact(2) {
        let px = u32::from(u16::from_le_bytes([c[0], c[1]]));
        rgba.extend([
            expand(px, fr, 0),
            expand(px, fg, 0),
            expand(px, fb, 0),
            expand(px, fa, 255),
        ]);
    }
    (TextureFormat::Rgba8UnormSrgb, rgba)
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
