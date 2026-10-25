//! Mip chains for block-compressed (DXT1/3/5) textures that ship without one (M-0058 phase C).
//!
//! 48,217 DXT files of the corpus have a single level, so they alias at a distance (the grainy
//! speckle on Köln's grass and asphalt). [`bc_chain`] decodes the base level, builds the
//! sRGB-correct chain of [`rgba8_srgb_chain`](super::rgba8_srgb_chain) and re-encodes levels
//! `1..` with `texpresso`'s range fit into the texture's own format. Level 0 keeps the file's
//! bytes, so the texture is unchanged up close and costs a third more video memory instead of
//! the 5–10× of an uncompressed chain. DXT1 cutout alpha survives (`texpresso` writes the
//! one-bit alpha block mode; `intel_tex_2`, measured in phase A, does not).
//!
//! Results are kept in the derived cache of [`super::chain_cache`]; see
//! [`prepare_block_compressed`].

use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

use bevy::image::Image;
use bevy::render::render_resource::TextureFormat;

use super::chain_cache::{self, ChainKey};
use super::dds_chain::chain_bytes;
use super::{decode_to_rgba8, full_chain_levels, level_size, looks_like_cutout, MipOptions};

/// Version of the chains [`bc_chain`] produces; bump on any change of its output so cached
/// chains are rebuilt.
pub const GENERATOR_VERSION: u32 = 1;

static BUILT: AtomicU64 = AtomicU64::new(0);
static CACHED: AtomicU64 = AtomicU64::new(0);
static MICROS: AtomicU64 = AtomicU64::new(0);

/// Running totals of [`prepare_block_compressed`] in this process (for load-time reports).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BcChainStats {
    /// Chains built (decoded, generated and encoded) because the cache had none.
    pub built: u64,
    /// Chains read from the derived cache.
    pub cached: u64,
    /// Time spent in both, in microseconds.
    pub micros: u64,
}

/// The totals so far.
#[must_use]
pub fn bc_chain_stats() -> BcChainStats {
    BcChainStats {
        built: BUILT.load(Relaxed),
        cached: CACHED.load(Relaxed),
        micros: MICROS.load(Relaxed),
    }
}

/// The `texpresso` format and block size of a block-compressed texture format.
fn block_format(format: TextureFormat) -> Option<(texpresso::Format, usize)> {
    match format {
        TextureFormat::Bc1RgbaUnormSrgb | TextureFormat::Bc1RgbaUnorm => {
            Some((texpresso::Format::Bc1, 8))
        }
        TextureFormat::Bc2RgbaUnormSrgb | TextureFormat::Bc2RgbaUnorm => {
            Some((texpresso::Format::Bc2, 16))
        }
        TextureFormat::Bc3RgbaUnormSrgb | TextureFormat::Bc3RgbaUnorm => {
            Some((texpresso::Format::Bc3, 16))
        }
        _ => None,
    }
}

/// The full chain (`level 0` as stored, levels `1..` generated and re-encoded) of a
/// single-level BC1/2/3 image and its level count. `None` for other images: other formats,
/// images that already have a chain, a 1×1 texture or data shorter than the base level.
#[must_use]
pub fn bc_chain(image: &Image) -> Option<(Vec<u8>, u32)> {
    let desc = &image.texture_descriptor;
    let (format, block_bytes) = block_format(desc.format)?;
    let (w, h) = (desc.size.width, desc.size.height);
    let levels = full_chain_levels(w, h);
    if desc.mip_level_count != 1 || desc.size.depth_or_array_layers != 1 || levels < 2 {
        return None;
    }
    let base_len = chain_bytes(w, h, 1, block_bytes);
    let base = image.data.as_deref()?.get(..base_len)?;
    let rgba = decode_to_rgba8(image)?;
    let pixels = rgba.data.as_deref()?.get(..w as usize * h as usize * 4)?;
    let options = MipOptions {
        alpha_cutoff: looks_like_cutout(pixels).then_some(0.5),
        weight_colour_by_alpha: false,
    };
    let chain = super::rgba8_srgb_chain(pixels, w, h, options)?;
    let params = texpresso::Params {
        algorithm: texpresso::Algorithm::RangeFit,
        ..Default::default()
    };
    let mut out = Vec::with_capacity(chain_bytes(w, h, levels, block_bytes));
    out.extend_from_slice(base);
    let mut offset = pixels.len();
    for level in 1..levels {
        let (lw, lh) = level_size(w, h, level);
        let (lw, lh) = (lw as usize, lh as usize);
        let src = chain.get(offset..offset + lw * lh * 4)?;
        offset += src.len();
        let start = out.len();
        out.resize(start + format.compressed_size(lw, lh), 0);
        format.compress(src, lw, lh, params, &mut out[start..]);
    }
    Some((out, levels))
}

/// Gives a single-level BC1/2/3 image its chain, from the derived cache when it holds one,
/// else built with [`bc_chain`] and stored. Returns `true` when levels were added.
pub fn prepare_block_compressed(image: &mut Image) -> bool {
    let desc = &image.texture_descriptor;
    let Some((_, block_bytes)) = block_format(desc.format) else {
        return false;
    };
    if desc.mip_level_count != 1 {
        return false;
    }
    let started = std::time::Instant::now();
    let (w, h) = (desc.size.width, desc.size.height);
    let base_len = chain_bytes(w, h, 1, block_bytes);
    let Some(base) = image.data.as_deref().and_then(|d| d.get(..base_len)) else {
        return false;
    };
    let key = ChainKey::new(desc.format, w, h, base, GENERATOR_VERSION);
    let cache = chain_cache::global();
    let (chain, levels, from_cache) = match cache.and_then(|c| c.get(&key, base)) {
        Some((chain, levels)) => (chain, levels, true),
        None => {
            let Some((chain, levels)) = bc_chain(image) else {
                return false;
            };
            if let Some(c) = cache {
                c.put(&key, &chain, levels);
            }
            (chain, levels, false)
        }
    };
    image.data = Some(chain);
    image.texture_descriptor.mip_level_count = levels;
    if from_cache {
        CACHED.fetch_add(1, Relaxed);
    } else {
        BUILT.fetch_add(1, Relaxed);
    }
    MICROS.fetch_add(started.elapsed().as_micros() as u64, Relaxed);
    true
}

#[cfg(test)]
#[path = "bc_mips/tests.rs"]
mod tests;
