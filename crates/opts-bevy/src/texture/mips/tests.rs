//! Tests of the mip-chain generation.

use super::*;
use bevy::asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension};

fn image(w: u32, h: u32, f: impl Fn(u32, u32) -> [u8; 4]) -> Image {
    let mut data = Vec::new();
    for y in 0..h {
        for x in 0..w {
            data.extend_from_slice(&f(x, y));
        }
    }
    Image::new(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

fn level_offset(w: u32, h: u32, level: u32) -> usize {
    (0..level)
        .map(|l| {
            let (lw, lh) = level_size(w, h, l);
            lw as usize * lh as usize * 4
        })
        .sum()
}

#[test]
fn chain_length_and_total_size_match_the_wgpu_layout() {
    for (w, h, levels) in [
        (256, 256, 9),
        (256, 64, 9),
        (1, 1, 1),
        (5, 3, 3),
        (533, 100, 10),
    ] {
        assert_eq!(full_chain_levels(w, h), levels, "{w}x{h}");
    }
    let mut img = image(64, 16, |_, _| [10, 20, 30, 255]);
    assert!(generate_mip_chain(&mut img, MipOptions::default()));
    assert_eq!(img.texture_descriptor.mip_level_count, 7);
    let expected: usize = (0..7)
        .map(|l| {
            let (w, h) = level_size(64, 16, l);
            (w * h * 4) as usize
        })
        .sum();
    assert_eq!(img.data.as_ref().map(Vec::len), Some(expected));
    // Level sizes: 64x16, 32x8, 16x4, 8x2, 4x1, 2x1, 1x1.
    assert_eq!(level_size(64, 16, 4), (4, 1));
    assert_eq!(level_size(64, 16, 6), (1, 1));
}

#[test]
fn black_white_checker_averages_to_188_not_128() {
    let mut img = image(8, 8, |x, y| {
        if (x + y) % 2 == 0 {
            [255, 255, 255, 255]
        } else {
            [0, 0, 0, 255]
        }
    });
    assert!(generate_mip_chain(&mut img, MipOptions::default()));
    let data = img.data.as_ref().expect("data");
    let l1 = level_offset(8, 8, 1);
    let v = data[l1];
    assert!((i32::from(v) - 188).abs() <= 1, "got {v}");
    assert_eq!(data[l1 + 3], 255);
    // The 1x1 level is the same mid grey.
    let last = level_offset(8, 8, 3);
    assert!((i32::from(data[last]) - 188).abs() <= 1);
}

#[test]
fn solid_colour_is_preserved_on_every_level_even_with_odd_sizes() {
    let mut img = image(7, 5, |_, _| [200, 100, 50, 255]);
    assert!(generate_mip_chain(&mut img, MipOptions::default()));
    let data = img.data.as_ref().expect("data");
    let total = data.len();
    assert_eq!(
        total,
        (0..3)
            .map(|l| {
                let (w, h) = level_size(7, 5, l);
                (w * h * 4) as usize
            })
            .sum::<usize>()
    );
    for px in data.chunks_exact(4) {
        for (got, want) in px.iter().zip([200u8, 100, 50, 255]) {
            assert!((i32::from(*got) - i32::from(want)).abs() <= 1, "{px:?}");
        }
    }
}

#[test]
fn non_power_of_two_sizes_work() {
    let mut img = image(533, 17, |x, _| [(x % 256) as u8, 0, 0, 255]);
    assert!(generate_mip_chain(&mut img, MipOptions::default()));
    assert_eq!(img.texture_descriptor.mip_level_count, 10);
    let expected: usize = (0..10)
        .map(|l| {
            let (w, h) = level_size(533, 17, l);
            (w * h * 4) as usize
        })
        .sum();
    assert_eq!(img.data.as_ref().map(Vec::len), Some(expected));
}

#[test]
fn alpha_is_the_plain_average_without_coverage_mode() {
    // Left half opaque, right half transparent: level 1 of a 2x1 image is the mean.
    let mut img = image(2, 1, |x, _| {
        if x == 0 {
            [255, 0, 0, 255]
        } else {
            [0, 255, 0, 0]
        }
    });
    assert!(generate_mip_chain(&mut img, MipOptions::default()));
    let data = img.data.as_ref().expect("data");
    let l1 = level_offset(2, 1, 1);
    assert!((i32::from(data[l1 + 3]) - 128).abs() <= 1);
    // Colour is the plain linear average of red and green.
    assert!(data[l1] > 150 && data[l1 + 1] > 150);
}

#[test]
fn alpha_weighting_keeps_the_colour_of_the_visible_texels() {
    let mut img = image(2, 1, |x, _| {
        if x == 0 {
            [255, 128, 0, 255]
        } else {
            [0, 0, 0, 0]
        }
    });
    let options = MipOptions {
        weight_colour_by_alpha: true,
        ..MipOptions::default()
    };
    assert!(generate_mip_chain(&mut img, options));
    let data = img.data.as_ref().expect("data");
    let l1 = level_offset(2, 1, 1);
    assert_eq!(&data[l1..l1 + 3], &[255, 128, 0]);
    assert!((i32::from(data[l1 + 3]) - 128).abs() <= 1);
}

#[test]
fn coverage_mode_keeps_the_share_above_the_cutoff() {
    // A thin vertical line (1 of 8 columns opaque): the plain average of level 2 is 0.25 alpha
    // at best, i.e. below the cutoff, so the line would vanish.
    let line = |x: u32, _| {
        if x == 3 {
            [0, 200, 0, 255]
        } else {
            [0, 100, 0, 0]
        }
    };
    let share = |data: &[u8], w: u32, h: u32, level: u32| {
        let off = level_offset(8, 8, level);
        let (lw, lh) = level_size(w, h, level);
        let px = &data[off..off + (lw * lh * 4) as usize];
        px.chunks_exact(4).filter(|p| p[3] >= 128).count() as f32 / (lw * lh) as f32
    };
    let mut plain = image(8, 8, line);
    generate_mip_chain(&mut plain, MipOptions::default());
    let mut cov = image(8, 8, line);
    generate_mip_chain(
        &mut cov,
        MipOptions {
            alpha_cutoff: Some(0.5),
            ..MipOptions::default()
        },
    );
    let (pd, cd) = (
        plain.data.as_ref().expect("d"),
        cov.data.as_ref().expect("d"),
    );
    assert!((share(cd, 8, 8, 0) - 0.125).abs() < 1e-6);
    assert_eq!(share(pd, 8, 8, 2), 0.0, "plain averaging loses the line");
    // Texel quantisation: a 2x2 level can only reach 0.25 steps, so it overshoots.
    let kept = share(cd, 8, 8, 2);
    assert!((0.125..=0.5).contains(&kept), "coverage kept: {kept}");
}

#[test]
fn cutout_detection_separates_foliage_from_glass() {
    let foliage = image(
        8,
        8,
        |x, _| if x < 4 { [0, 90, 0, 255] } else { [0, 0, 0, 0] },
    );
    let edge = image(8, 8, |x, _| match x {
        0..=3 => [0, 90, 0, 255],
        4 => [0, 90, 0, 128],
        _ => [0, 0, 0, 0],
    });
    let glass = image(8, 8, |_, _| [50, 60, 70, 100]);
    let solid = image(8, 8, |_, _| [50, 60, 70, 255]);
    let data = |i: &Image| i.data.clone().expect("data");
    assert!(looks_like_cutout(&data(&foliage)));
    assert!(looks_like_cutout(&data(&edge)));
    assert!(!looks_like_cutout(&data(&glass)));
    assert!(!looks_like_cutout(&data(&solid)));
}

#[test]
fn textures_with_a_chain_masks_and_compressed_formats_are_untouched() {
    let mut img = image(4, 4, |_, _| [1, 2, 3, 4]);
    img.texture_descriptor.mip_level_count = 3;
    assert!(!generate_mip_chain(&mut img, MipOptions::default()));
    let mut mask = image(4, 4, |_, _| [1, 2, 3, 4]);
    mask.texture_descriptor.format = TextureFormat::R8Unorm;
    assert!(!generate_mip_chain(&mut mask, MipOptions::default()));
    let mut bc = image(4, 4, |_, _| [1, 2, 3, 4]);
    bc.texture_descriptor.format = TextureFormat::Bc1RgbaUnormSrgb;
    assert!(!generate_mip_chain(&mut bc, MipOptions::default()));
    let mut one = image(1, 1, |_, _| [1, 2, 3, 4]);
    assert!(!generate_mip_chain(&mut one, MipOptions::default()));
    let mut short = image(4, 4, |_, _| [1, 2, 3, 4]);
    short.data = Some(vec![0; 10]);
    assert!(!generate_mip_chain(&mut short, MipOptions::default()));
}
