//! Integration tests for texture decoding, format sniffing, and material bridging.

#![deny(unsafe_code)]

use bevy::app::App;
use bevy::asset::AssetPlugin;
use bevy::pbr::StandardMaterial;
use bevy::prelude::AlphaMode;
use bevy::render::render_resource::TextureFormat;
use opts_bevy::{
    dds_to_bevy_image, load_bevy_image, o3d_material_to_standard_material, sniff_image_format,
    specular_power_to_roughness, x_material_to_standard_material, ImageFormat, OptsAssetPlugin,
    OptsMaterialPlugin,
};
use opts_core::formats::o3d::Material as O3dMaterial;
use opts_core::formats::x::XMaterial;

/// Helper generating a minimal synthetic DDS file header with given parameters.
fn build_synthetic_dds_buffer(
    width: u32,
    height: u32,
    fourcc: Option<[u8; 4]>,
    is_a8: bool,
    payload: &[u8],
) -> Vec<u8> {
    let mut buf = Vec::with_capacity(128 + payload.len());
    // Magic 'DDS '
    buf.extend_from_slice(b"DDS ");
    // dwSize = 124
    buf.extend_from_slice(&124u32.to_le_bytes());
    // dwFlags: CAPS | HEIGHT | WIDTH | PIXELFORMAT (0x1007)
    let flags: u32 = 0x1007;
    buf.extend_from_slice(&flags.to_le_bytes());
    // dwHeight, dwWidth
    buf.extend_from_slice(&height.to_le_bytes());
    buf.extend_from_slice(&width.to_le_bytes());
    // dwPitchOrLinearSize
    buf.extend_from_slice(&0u32.to_le_bytes());
    // dwDepth
    buf.extend_from_slice(&0u32.to_le_bytes());
    // dwMipMapCount
    buf.extend_from_slice(&0u32.to_le_bytes());
    // dwReserved1[11]
    buf.extend_from_slice(&[0u8; 44]);

    // DDS_PIXELFORMAT (32 bytes)
    // dwSize = 32
    buf.extend_from_slice(&32u32.to_le_bytes());
    if let Some(fcc) = fourcc {
        // dwFlags = DDPF_FOURCC (0x4)
        buf.extend_from_slice(&0x4u32.to_le_bytes());
        buf.extend_from_slice(&fcc);
        // dwRGBBitCount, dwRBitMask, dwGBitMask, dwBBitMask, dwABitMask
        buf.extend_from_slice(&[0u8; 20]);
    } else if is_a8 {
        // dwFlags = DDPF_ALPHA (0x2)
        buf.extend_from_slice(&0x2u32.to_le_bytes());
        // dwFourCC = 0
        buf.extend_from_slice(&0u32.to_le_bytes());
        // dwRGBBitCount = 8
        buf.extend_from_slice(&8u32.to_le_bytes());
        // dwRBitMask, dwGBitMask, dwBBitMask = 0
        buf.extend_from_slice(&[0u8; 12]);
        // dwABitMask = 0xFF
        buf.extend_from_slice(&0xFFu32.to_le_bytes());
    } else {
        // Uncompressed RGBA (32-bit)
        // dwFlags = DDPF_RGB | DDPF_ALPHAPIXELS (0x41)
        buf.extend_from_slice(&0x41u32.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes());
        buf.extend_from_slice(&32u32.to_le_bytes());
        buf.extend_from_slice(&0x00FF_0000u32.to_le_bytes()); // R
        buf.extend_from_slice(&0x0000_FF00u32.to_le_bytes()); // G
        buf.extend_from_slice(&0x0000_00FFu32.to_le_bytes()); // B
        buf.extend_from_slice(&0xFF00_0000u32.to_le_bytes()); // A
    }

    // dwCaps: DDSCAPS_TEXTURE (0x1000)
    buf.extend_from_slice(&0x1000u32.to_le_bytes());
    // dwCaps2, dwCaps3, dwCaps4, dwReserved2
    buf.extend_from_slice(&[0u8; 16]);

    // Append pixel data payload
    buf.extend_from_slice(payload);
    buf
}

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

#[test]
fn test_bmp_content_sniffed_via_unified_loader() {
    // Generate a minimal 2x2 BMP image using the image crate
    let img_buf = image::RgbaImage::new(2, 2);
    let mut bmp_bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut bmp_bytes);
    img_buf
        .write_to(&mut cursor, image::ImageFormat::Bmp)
        .expect("write to BMP");

    // Unified loader must sniff 'BM' and decode correctly into Bevy Image
    let img = load_bevy_image(&bmp_bytes).expect("load BMP image");
    assert_eq!(img.texture_descriptor.size.width, 2);
    assert_eq!(img.texture_descriptor.size.height, 2);
    assert_eq!(img.texture_descriptor.format, TextureFormat::Rgba8UnormSrgb);
}

#[test]
fn test_png_content_sniffed_via_unified_loader() {
    let img_buf = image::RgbaImage::new(3, 3);
    let mut png_bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut png_bytes);
    img_buf
        .write_to(&mut cursor, image::ImageFormat::Png)
        .expect("write to PNG");

    let img = load_bevy_image(&png_bytes).expect("load PNG image");
    assert_eq!(img.texture_descriptor.size.width, 3);
    assert_eq!(img.texture_descriptor.size.height, 3);
}

#[test]
fn test_o3d_and_x_material_bridge() {
    let o3d_mat = O3dMaterial {
        diffuse: [1.0, 0.5, 0.0, 1.0],
        specular: [0.5, 0.5, 0.5],
        emissive: [0.2, 0.0, 0.0],
        power: 20.0,
        textures: vec!["exterior.dds".into()],
    };

    let std_mat: StandardMaterial = o3d_material_to_standard_material(&o3d_mat, None);
    assert_eq!(std_mat.alpha_mode, AlphaMode::Opaque);
    assert_eq!(std_mat.metallic, 0.0);
    assert!(std_mat.perceptual_roughness < 1.0);
    assert!(std_mat.perceptual_roughness > 0.0);

    let x_mat = XMaterial {
        name: Some("tinted_window".into()),
        diffuse: [0.1, 0.1, 0.2, 0.5],
        specular: [1.0, 1.0, 1.0],
        emissive: [0.0, 0.0, 0.0],
        power: 100.0,
        texture_filename: Some("glass.tga".into()),
    };

    let std_mat_x: StandardMaterial = x_material_to_standard_material(&x_mat, None);
    assert_eq!(std_mat_x.alpha_mode, AlphaMode::Blend);
    assert!(std_mat_x.perceptual_roughness < 0.2);
}

#[test]
fn test_specular_power_to_roughness() {
    assert_eq!(specular_power_to_roughness(0.0), 1.0);
    assert_eq!(specular_power_to_roughness(-5.0), 1.0);
    let r1 = specular_power_to_roughness(10.0);
    let r2 = specular_power_to_roughness(50.0);
    assert!(r1 > r2); // Higher power -> lower roughness (glossier)
}

#[test]
fn test_opts_material_plugin_app_registration() {
    let mut app = App::new();
    app.add_plugins(AssetPlugin {
        watch_for_changes_override: Some(false),
        ..Default::default()
    });
    app.add_plugins(OptsMaterialPlugin);

    let mut unified_app = App::new();
    unified_app.add_plugins(AssetPlugin {
        watch_for_changes_override: Some(false),
        ..Default::default()
    });
    unified_app.add_plugins(OptsAssetPlugin);
    // Verify plugins register without panics
}
