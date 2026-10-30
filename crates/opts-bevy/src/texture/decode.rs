//! Generic image container loaders (`.bmp`, `.tga`, `.png`, `.jpg`) and CPU BC1/BC2/BC3 decoding.

use super::*;

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
pub(super) fn decode_with(
    bytes: &[u8],
    format: Option<image::ImageFormat>,
) -> Result<Image, TextureError> {
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

/// The 16 alpha values of a BC2 (explicit 4-bit) or BC3 (interpolated) block; opaque for BC1.
fn block_alpha(block: &[u8], kind: u8) -> [u8; 16] {
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
    alpha
}

/// The four colours a BC block's 2-bit indices choose from (the fourth is transparent in a BC1
/// block with `c0 <= c1`).
fn colour_palette(colour: &[u8], kind: u8) -> [[u32; 4]; 4] {
    let c0 = u16::from_le_bytes([colour[0], colour[1]]);
    let c1 = u16::from_le_bytes([colour[2], colour[3]]);
    let expand = |c: u16| {
        let (r, g, b) = (
            u32::from((c >> 11) & 31),
            u32::from((c >> 5) & 63),
            u32::from(c & 31),
        );
        [r * 255 / 31, g * 255 / 63, b * 255 / 31]
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
    palette
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
            // Loaded textures carry their mip chain after level 0 (`prepare_object_texture`);
            // demanding exactly one level made every TGA/BMP base or transparency map fail, so
            // `[matl_transmap]` was ignored for them (Aachen's parked cars drawn see-through).
            if data.len() < w * h * 4 {
                return None;
            }
            let mut level0 = image.clone();
            if let Some(d) = level0.data.as_mut() {
                d.truncate(w * h * 4);
            }
            level0.texture_descriptor.mip_level_count = 1;
            return Some(level0);
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
            let alpha = block_alpha(block, kind);
            let palette = colour_palette(colour, kind);
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
