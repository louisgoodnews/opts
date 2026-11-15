//! Decoding one texture, building its chain and encoding its levels with each encoder.

use std::path::Path;
use std::time::Instant;

use bevy::render::render_resource::TextureFormat;
use opts_bevy::texture::{
    dds_to_bevy_image, decode_to_rgba8, full_chain_levels, generate_mip_chain, level_size,
    looks_like_cutout, MipOptions,
};

use crate::{Totals, ENCODERS};

/// Decodes one texture, builds its chain and measures the encoders on levels 1..; skips files
/// that are not a single-level DXT1/3/5 texture.
pub(crate) fn measure(t: &mut Totals, path: &Path) {
    let Ok(bytes) = std::fs::read(path) else {
        return;
    };
    let Ok(image) = dds_to_bevy_image(&bytes) else {
        return;
    };
    let format = image.texture_descriptor.format;
    let tex_format = match format {
        TextureFormat::Bc1RgbaUnormSrgb => texpresso::Format::Bc1,
        TextureFormat::Bc2RgbaUnormSrgb => texpresso::Format::Bc2,
        TextureFormat::Bc3RgbaUnormSrgb => texpresso::Format::Bc3,
        _ => return,
    };
    if image.texture_descriptor.mip_level_count != 1 {
        return;
    }
    let (w, h) = (image.width(), image.height());
    if w.max(h) < 2 {
        return;
    }

    let started = Instant::now();
    let Some(mut rgba) = decode_to_rgba8(&image) else {
        return;
    };
    t.decode_s += started.elapsed().as_secs_f64();
    let cutout = rgba.data.as_deref().is_some_and(looks_like_cutout);
    let started = Instant::now();
    generate_mip_chain(
        &mut rgba,
        MipOptions {
            alpha_cutoff: cutout.then_some(0.5),
            weight_colour_by_alpha: false,
        },
    );
    t.chain_s += started.elapsed().as_secs_f64();
    let chain = rgba.data.as_deref().unwrap_or_default();
    let has_alpha = chain[..(w * h * 4) as usize]
        .chunks_exact(4)
        .any(|p| p[3] < 255);
    if tex_format == texpresso::Format::Bc1 && has_alpha {
        t.bc1_with_alpha += 1;
    }
    t.textures += 1;
    t.base_pixels += u64::from(w) * u64::from(h);
    encode_levels(t, tex_format, chain, (w, h), has_alpha);
}

/// Re-encodes levels 1.. of the chain with every encoder and accumulates time and error.
pub(crate) fn encode_levels(
    t: &mut Totals,
    tex_format: texpresso::Format,
    chain: &[u8],
    size: (u32, u32),
    has_alpha: bool,
) {
    let (w, h) = size;
    let bc1 = tex_format == texpresso::Format::Bc1;
    let mut offset = (w * h * 4) as usize;
    for level in 1..full_chain_levels(w, h) {
        let (lw, lh) = level_size(w, h, level);
        let (lw, lh) = (lw as usize, lh as usize);
        let src = &chain[offset..offset + lw * lh * 4];
        offset += lw * lh * 4;
        t.chain_pixels += (lw * lh) as u64;
        for e in 0..ENCODERS.len() {
            let started = Instant::now();
            let params = texpresso::Params {
                algorithm: if e == 0 {
                    texpresso::Algorithm::RangeFit
                } else {
                    texpresso::Algorithm::ClusterFit
                },
                ..Default::default()
            };
            let mut blocks = vec![0u8; tex_format.compressed_size(lw, lh)];
            tex_format.compress(src, lw, lh, params, &mut blocks);
            t.encode_s[e] += started.elapsed().as_secs_f64();
            let mut decoded = vec![0u8; lw * lh * 4];
            tex_format.decompress(&blocks, lw, lh, &mut decoded);
            for (a, b) in src.chunks_exact(4).zip(decoded.chunks_exact(4)) {
                for c in 0..3 {
                    let d = f64::from(a[c]) - f64::from(b[c]);
                    t.sq_err[e] += d * d;
                }
                if has_alpha {
                    let d = f64::from(a[3]) - f64::from(b[3]);
                    t.alpha_sq_err[usize::from(!bc1)][e] += d * d;
                }
            }
        }
        if has_alpha {
            t.alpha_samples[usize::from(!bc1)] += (lw * lh) as u64;
        }
    }
}
