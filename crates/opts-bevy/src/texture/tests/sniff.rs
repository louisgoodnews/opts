//! Format sniffing: the magic bytes, and an empty buffer.

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
