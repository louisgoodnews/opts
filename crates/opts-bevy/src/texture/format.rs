//! Content sniffing of an image buffer: the container format from its magic bytes.

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
