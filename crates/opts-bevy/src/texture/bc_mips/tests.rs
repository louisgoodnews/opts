use super::*;
use crate::texture::dds_to_bevy_image;
use bevy::render::render_resource::{Extent3d, TextureDimension};

/// A checker of `cell`-texel squares: colour `a` and `b`; `b` transparent when `cutout`.
fn checker(w: usize, h: usize, cell: usize, cutout: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            if (x / cell + y / cell).is_multiple_of(2) {
                out.extend([200, 60, 30, 255]);
            } else if cutout {
                out.extend([0, 0, 0, 0]);
            } else {
                out.extend([20, 120, 220, 255]);
            }
        }
    }
    out
}

fn bc_image(format: TextureFormat, w: u32, h: u32, rgba: &[u8]) -> Image {
    let (tex, _) = block_format(format).expect("bc format");
    let mut data = vec![0u8; tex.compressed_size(w as usize, h as usize)];
    tex.compress(
        rgba,
        w as usize,
        h as usize,
        texpresso::Params::default(),
        &mut data,
    );
    let mut image = Image::default();
    image.texture_descriptor.size = Extent3d {
        width: w,
        height: h,
        depth_or_array_layers: 1,
    };
    image.texture_descriptor.format = format;
    image.texture_descriptor.dimension = TextureDimension::D2;
    image.texture_descriptor.mip_level_count = 1;
    image.data = Some(data);
    image
}

fn levels_of(tex: texpresso::Format, chain: &[u8], w: u32, h: u32, levels: u32) -> Vec<Vec<u8>> {
    let mut offset = 0;
    (0..levels)
        .map(|l| {
            let (lw, lh) = level_size(w, h, l);
            let (lw, lh) = (lw as usize, lh as usize);
            let len = tex.compressed_size(lw, lh);
            let mut rgba = vec![0u8; lw * lh * 4];
            tex.decompress(&chain[offset..offset + len], lw, lh, &mut rgba);
            offset += len;
            rgba
        })
        .collect()
}

#[test]
fn every_format_gets_a_full_chain_with_the_gpu_layout_and_level_0_unchanged() {
    for (format, bb) in [
        (TextureFormat::Bc1RgbaUnormSrgb, 8),
        (TextureFormat::Bc2RgbaUnormSrgb, 16),
        (TextureFormat::Bc3RgbaUnormSrgb, 16),
    ] {
        // Power of two, non-power-of-two with levels below the block size, and a thin strip.
        for (w, h) in [(64, 64), (520, 300), (12, 4)] {
            let image = bc_image(format, w, h, &checker(w as usize, h as usize, 4, false));
            let base = image.data.clone().expect("data");
            let (chain, levels) = bc_chain(&image).expect("chain");
            assert_eq!(levels, full_chain_levels(w, h), "{format:?} {w}x{h}");
            assert_eq!(
                chain.len(),
                chain_bytes(w, h, levels, bb),
                "{format:?} {w}x{h}"
            );
            assert!(chain.starts_with(&base));
        }
    }
}

#[test]
fn generated_levels_average_the_base_level() {
    let (w, h) = (64, 64);
    let image = bc_image(
        TextureFormat::Bc1RgbaUnormSrgb,
        w,
        h,
        &checker(64, 64, 1, false),
    );
    let (chain, levels) = bc_chain(&image).expect("chain");
    let decoded = levels_of(texpresso::Format::Bc1, &chain, w, h, levels);
    // A one-texel checker averages to one flat colour from level 1 on (in linear light).
    let last = &decoded[3];
    let first = &last[..4];
    assert!(last.chunks_exact(4).all(|p| p == first), "level 3 not flat");
    assert!(first[0] > 100 && first[2] > 100, "average colour {first:?}");
}

#[test]
fn dxt1_cutout_alpha_survives_the_chain() {
    let (w, h) = (64, 64);
    let image = bc_image(
        TextureFormat::Bc1RgbaUnormSrgb,
        w,
        h,
        &checker(64, 64, 8, true),
    );
    let (chain, levels) = bc_chain(&image).expect("chain");
    let decoded = levels_of(texpresso::Format::Bc1, &chain, w, h, levels);
    for (level, px) in decoded.iter().enumerate().skip(1).take(2) {
        let clear = px.chunks_exact(4).filter(|p| p[3] == 0).count() as f64;
        let share = clear / (px.len() / 4) as f64;
        assert!((0.35..=0.65).contains(&share), "level {level}: {share}");
    }
}

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
