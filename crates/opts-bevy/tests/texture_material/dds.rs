//! Format sniffing and synthetic DDS decoding.

use super::synthetic::build_synthetic_dds_buffer;
use super::*;

#[test]
fn test_sniff_image_formats() {
    assert_eq!(
        sniff_image_format(b"DDS \x7c\x00\x00\x00"),
        ImageFormat::Dds
    );
    assert_eq!(sniff_image_format(b"BM\x36\x00\x00\x00"), ImageFormat::Bmp);
    assert_eq!(
        sniff_image_format(b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR"),
        ImageFormat::Png
    );
    assert_eq!(
        sniff_image_format(&[0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10]),
        ImageFormat::Jpeg
    );
    assert_eq!(sniff_image_format(b"GARBAGE_BYTES"), ImageFormat::Unknown);
}

#[test]
fn test_synthetic_dxt1_dds_decode() {
    // 4x4 DXT1 is 1 block = 8 bytes
    let payload = [0x55; 8];
    let dds_buf = build_synthetic_dds_buffer(4, 4, Some(*b"DXT1"), false, &payload);

    let img = dds_to_bevy_image(&dds_buf).expect("failed to decode synthetic DXT1 DDS");
    assert_eq!(img.texture_descriptor.size.width, 4);
    assert_eq!(img.texture_descriptor.size.height, 4);
    assert_eq!(
        img.texture_descriptor.format,
        TextureFormat::Bc1RgbaUnormSrgb
    );
}

#[test]
fn test_synthetic_dxt5_dds_decode() {
    // 4x4 DXT5 is 1 block = 16 bytes
    let payload = [0xAA; 16];
    let dds_buf = build_synthetic_dds_buffer(4, 4, Some(*b"DXT5"), false, &payload);

    let img = dds_to_bevy_image(&dds_buf).expect("failed to decode synthetic DXT5 DDS");
    assert_eq!(img.texture_descriptor.size.width, 4);
    assert_eq!(img.texture_descriptor.size.height, 4);
    assert_eq!(
        img.texture_descriptor.format,
        TextureFormat::Bc3RgbaUnormSrgb
    );
}

#[test]
fn test_synthetic_a8_alpha_mask_dds_decode() {
    // 4x4 A8 is 16 bytes (1 byte per pixel)
    let payload = [0x80; 16];
    let dds_buf = build_synthetic_dds_buffer(4, 4, None, true, &payload);

    let img = dds_to_bevy_image(&dds_buf).expect("failed to decode synthetic A8 DDS");
    assert_eq!(img.texture_descriptor.size.width, 4);
    assert_eq!(img.texture_descriptor.size.height, 4);
    assert_eq!(img.texture_descriptor.format, TextureFormat::R8Unorm);
}
