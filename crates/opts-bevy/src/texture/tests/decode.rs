//! Decoding of TGA headers, RGBA8 mip chains, BC3 blocks and 16-bit DDS pixels.

use super::*;

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

/// A TGA/BMP texture as the loader keeps it (level 0 plus its mip chain) still decodes to its
/// first level: the transparency-map composition needs it (Aachen's parked cars were drawn
/// see-through because the base texture was refused).
#[test]
fn an_rgba8_texture_with_its_mip_chain_decodes_to_level_0() {
    let mut texels = Vec::new();
    for i in 0..16u8 {
        texels.extend_from_slice(&[i, 2 * i, 3 * i, 255 - i]);
    }
    let mut img = Image::new(
        Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        texels.clone(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    crate::texture_filter::prepare_object_texture(&mut img);
    assert!(
        img.texture_descriptor.mip_level_count > 1,
        "mips were built"
    );
    let out = decode_to_rgba8(&img).expect("decodes");
    assert_eq!(out.texture_descriptor.mip_level_count, 1);
    assert_eq!(out.data.as_deref(), Some(&texels[..]));
}

#[test]
fn sixteen_bit_dds_pixels_expand_to_rgba8() {
    // R5G6B5: pure red, pure green, pure blue, white.
    let px = |v: u16| v.to_le_bytes();
    let data: Vec<u8> = [0xF800u16, 0x07E0, 0x001F, 0xFFFF]
        .iter()
        .flat_map(|v| px(*v))
        .collect();
    let (fmt, rgba) = convert_rgb16_to_rgba(&data, [0xF800, 0x07E0, 0x001F, 0]);
    assert_eq!(fmt, TextureFormat::Rgba8UnormSrgb);
    assert_eq!(&rgba[0..4], &[255, 0, 0, 255]);
    assert_eq!(&rgba[4..8], &[0, 255, 0, 255]);
    assert_eq!(&rgba[8..12], &[0, 0, 255, 255]);
    assert_eq!(&rgba[12..16], &[255, 255, 255, 255]);
    // A4R4G4B4 with half alpha, red.
    let data = px(0x8F00).to_vec();
    let (_, rgba) = convert_rgb16_to_rgba(&data, [0x0F00, 0x00F0, 0x000F, 0xF000]);
    assert_eq!(&rgba[0..4], &[255, 0, 0, 136]);
    // A1R5G5B5 transparent black.
    let data = px(0x0000).to_vec();
    let (_, rgba) = convert_rgb16_to_rgba(&data, [0x7C00, 0x03E0, 0x001F, 0x8000]);
    assert_eq!(&rgba[0..4], &[0, 0, 0, 0]);
}
