//! The layout of a mip chain: its length and sizes, the averages of its levels and the textures left alone.

use super::*;

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
