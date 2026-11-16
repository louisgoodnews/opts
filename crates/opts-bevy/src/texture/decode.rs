//! Generic image container loaders (`.bmp`, `.tga`, `.png`, `.jpg`).

mod bc;

pub use bc::decode_to_rgba8;

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
