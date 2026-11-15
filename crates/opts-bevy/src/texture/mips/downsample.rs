//! Box downsampling of a level by 2x2 texels, in plain bytes and in linear light.

use super::*;

/// Integer fast path of the most common step: plain 2x2 mean of a level whose width and height
/// both halve exactly (colour averaged in linear light, alpha as the plain mean).
pub(super) fn downsample_2x2_plain(src: &[u8], sw: usize, dw: usize, dh: usize) -> Vec<u8> {
    let (lin, enc) = (srgb_to_linear16(), linear14_to_srgb());
    let mut out = vec![0u8; dw * dh * 4];
    for (y, orow) in out.chunks_exact_mut(dw * 4).enumerate() {
        let r0 = &src[(2 * y) * sw * 4..][..sw * 4];
        let r1 = &src[(2 * y + 1) * sw * 4..][..sw * 4];
        for ((o, a), b) in orow
            .chunks_exact_mut(4)
            .zip(r0.chunks_exact(8))
            .zip(r1.chunks_exact(8))
        {
            for c in 0..3 {
                let sum = u32::from(lin[a[c] as usize])
                    + u32::from(lin[a[c + 4] as usize])
                    + u32::from(lin[b[c] as usize])
                    + u32::from(lin[b[c + 4] as usize]);
                // mean (sum / 4) in 16 bit -> 14 bit index, rounded.
                o[c] = enc[(((sum + 8) >> 4) as usize).min(ENC_STEPS)];
            }
            let alpha = u32::from(a[3]) + u32::from(a[7]) + u32::from(b[3]) + u32::from(b[7]);
            o[3] = ((alpha + 2) >> 2) as u8;
        }
    }
    out
}

/// Fast path for the common case that each axis shrinks by exactly 1 or 2: every destination
/// texel is the mean of a 1/2 x 1/2 block (the exact area weights of [`downsample_general`]).
pub(super) fn downsample_exact(
    src: &[u8],
    sw: u32,
    dw: u32,
    dh: u32,
    k: (usize, usize),
    weighted: bool,
) -> Vec<u8> {
    let lut = srgb_to_linear_lut();
    let (kx, ky) = k;
    let sw = sw as usize;
    let mut out = vec![0u8; (dw * dh * 4) as usize];
    for y in 0..dh as usize {
        for x in 0..dw as usize {
            let (mut acc, mut plain) = ([0.0f32; 3], [0.0f32; 3]);
            let (mut alpha, mut wsum) = (0.0f32, 0.0f32);
            for j in 0..ky {
                let row = ((y * ky + j) * sw + x * kx) * 4;
                for p in src[row..row + kx * 4].chunks_exact(4) {
                    let a = f32::from(p[3]);
                    let w = if weighted { a } else { 1.0 };
                    alpha += a;
                    wsum += w;
                    for c in 0..3 {
                        let l = lut[p[c] as usize];
                        acc[c] += l * w;
                        plain[c] += l;
                    }
                }
            }
            let n = (kx * ky) as f32;
            let o = (y * dw as usize + x) * 4;
            let (sum, div) = if wsum > 1e-6 { (acc, wsum) } else { (plain, n) };
            for c in 0..3 {
                out[o + c] = encode_srgb(sum[c] / div);
            }
            out[o + 3] = (alpha / n + 0.5) as u8;
        }
    }
    out
}

/// Builds the next level from `src` (`sw` x `sh` RGBA8 sRGB) with exact area weights.
pub(super) fn downsample(src: &[u8], sw: u32, sh: u32, dw: u32, dh: u32, weighted: bool) -> Vec<u8> {
    let exact = |s: u32, d: u32| (s == d || s == 2 * d).then_some((s / d) as usize);
    if let (Some(kx), Some(ky)) = (exact(sw, dw), exact(sh, dh)) {
        if !weighted && kx == 2 && ky == 2 {
            return downsample_2x2_plain(src, sw as usize, dw as usize, dh as usize);
        }
        return downsample_exact(src, sw, dw, dh, (kx, ky), weighted);
    }
    downsample_general(src, sw, sh, dw, dh, weighted)
}
