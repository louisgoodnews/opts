//! Tests of format sniffing, DDS decoding and TGA repair.

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
fn test_non_block_aligned_mip_chain_is_relaid_out() {
    // Dimensions like hochspannungsmast.dds (256x1025, 11 mips). Rounding 1025 to 1028 shifts
    // the intermediate level sizes (the GPU expects 351376 bytes, the file holds 350608); the
    // chain is re-laid out instead of dropped (Q-TQ03).
    let bytes = dxt5_dds(256, 1025, 11);
    assert_eq!(bytes.len(), 128 + 350608);
    let before = dropped_dds_chains();
    let img = dds_to_bevy_image(&bytes).expect("decodes without panic");
    assert_eq!(img.texture_descriptor.size.width, 256);
    assert_eq!(img.texture_descriptor.size.height, 1028);
    assert_eq!(img.texture_descriptor.mip_level_count, 11);
    assert_eq!(img.data.as_ref().unwrap().len(), 351376);
    assert!(dropped_dds_chains() >= before);
}

#[test]
fn a_chain_shorter_than_declared_is_still_dropped_and_counted() {
    let mut bytes = dxt5_dds(64, 64, 3);
    bytes[28..32].copy_from_slice(&7u32.to_le_bytes());
    let before = dropped_dds_chains();
    let img = dds_to_bevy_image(&bytes).expect("decodes");
    assert_eq!(img.texture_descriptor.mip_level_count, 1);
    assert!(dropped_dds_chains() > before);
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

mod tga_repair {
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
