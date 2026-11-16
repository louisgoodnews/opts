//! Images that get no chain, and DDS files whose size is not a multiple of 4.

use super::*;
use crate::texture::dds_to_bevy_image;

#[test]
fn images_with_a_chain_other_formats_and_single_texels_are_left_alone() {
    let mut image = bc_image(
        TextureFormat::Bc1RgbaUnormSrgb,
        8,
        8,
        &checker(8, 8, 2, false),
    );
    image.texture_descriptor.mip_level_count = 2;
    assert!(bc_chain(&image).is_none());
    let rgba = Image::default();
    assert!(bc_chain(&rgba).is_none());
    let short = {
        let mut i = bc_image(
            TextureFormat::Bc3RgbaUnormSrgb,
            8,
            8,
            &checker(8, 8, 2, false),
        );
        i.data.as_mut().expect("data").truncate(10);
        i
    };
    assert!(bc_chain(&short).is_none());
}

#[test]
fn a_dds_whose_size_is_not_a_multiple_of_4_gets_a_chain_for_the_rounded_extent() {
    // A 9x6 DXT1 file: the loader rounds the extent to 12x8.
    let mut dds = ddsfile::Dds::new_d3d(ddsfile::NewD3dParams {
        height: 6,
        width: 9,
        depth: None,
        format: ddsfile::D3DFormat::DXT1,
        mipmap_levels: None,
        caps2: None,
    })
    .expect("dds");
    let blocks = 3 * 2;
    let rgba = checker(12, 8, 2, false);
    let mut data = vec![0u8; blocks * 8];
    texpresso::Format::Bc1.compress(&rgba, 12, 8, texpresso::Params::default(), &mut data);
    dds.data = data;
    let mut bytes = Vec::new();
    dds.write(&mut bytes).expect("write dds");
    let image = dds_to_bevy_image(&bytes).expect("decode");
    assert_eq!((image.width(), image.height()), (12, 8));
    let (chain, levels) = bc_chain(&image).expect("chain");
    assert_eq!(levels, 4);
    assert_eq!(chain.len(), chain_bytes(12, 8, 4, 8));
}
