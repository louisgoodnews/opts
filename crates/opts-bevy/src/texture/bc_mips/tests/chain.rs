//! The mip chain of BC-compressed images: full chains, averaged levels and cutout alpha.

use super::*;

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
