//! Alpha-cutout textures: the coverage of a level, its match to the source, and the chain that keeps it.

use super::*;

pub(super) fn coverage(level: &[u8], cutoff: f32, scale: f32) -> f32 {
    let n = level.len() / 4;
    if n == 0 {
        return 0.0;
    }
    let hit = level
        .chunks_exact(4)
        .filter(|p| f32::from(p[3]) / 255.0 * scale >= cutoff)
        .count();
    hit as f32 / n as f32
}

/// Scales the alpha of `level` so its coverage at `cutoff` reaches `target` (levels that kept
/// enough coverage are left alone).
pub(super) fn match_coverage(level: &mut [u8], cutoff: f32, target: f32) {
    if target <= 0.0 || coverage(level, cutoff, 1.0) >= target - 0.002 {
        return;
    }
    let (mut lo, mut hi) = (1.0f32, 16.0f32);
    for _ in 0..16 {
        let mid = 0.5 * (lo + hi);
        if coverage(level, cutoff, mid) < target - 0.002 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    for p in level.chunks_exact_mut(4) {
        p[3] = (f32::from(p[3]) * hi).clamp(0.0, 255.0).round() as u8;
    }
}

/// Builds all levels below the base of an RGBA8 sRGB image and returns them appended after the
/// base level (the layout of [`Image::data`]). `None` when the base has the wrong length.
#[must_use]
pub fn rgba8_srgb_chain(
    base: &[u8],
    width: u32,
    height: u32,
    options: MipOptions,
) -> Option<Vec<u8>> {
    if width == 0 || height == 0 || base.len() != width as usize * height as usize * 4 {
        return None;
    }
    let levels = full_chain_levels(width, height);
    let target = options.alpha_cutoff.map(|c| coverage(base, c, 1.0));
    let mut out = Vec::with_capacity(base.len() + base.len() / 3 + 16);
    out.extend_from_slice(base);
    let mut prev: Option<Vec<u8>> = None;
    let (mut pw, mut ph) = (width, height);
    for level in 1..levels {
        let (nw, nh) = level_size(width, height, level);
        let mut next = downsample(
            prev.as_deref().unwrap_or(base),
            pw,
            ph,
            nw,
            nh,
            options.alpha_cutoff.is_some() || options.weight_colour_by_alpha,
        );
        if let (Some(cutoff), Some(target)) = (options.alpha_cutoff, target) {
            match_coverage(&mut next, cutoff, target);
        }
        out.extend_from_slice(&next);
        prev = Some(next);
        (pw, ph) = (nw, nh);
    }
    Some(out)
}

/// Whether the alpha channel looks like a hard cutout (foliage, fences, signs): of the texels
/// that are not opaque, at least two thirds are clearly transparent (alpha <= 16) and the rest is
/// the anti-aliased edge. Smooth translucency (glass, smoke) is not a cutout.
///
/// Provisional heuristic of the loader, which creates a texture before it knows the material's
/// alpha mode; M-0059 replaces it with the material's real cutoff.
#[must_use]
pub fn looks_like_cutout(rgba: &[u8]) -> bool {
    let (mut clear, mut partial) = (0usize, 0usize);
    for p in rgba.chunks_exact(4) {
        match p[3] {
            0..=16 => clear += 1,
            17..=239 => partial += 1,
            _ => {}
        }
    }
    clear > 0 && partial * 2 <= clear
}
