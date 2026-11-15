//! Downsampling of an arbitrary size to the next level (the general case of [`downsample`]).

use super::*;

/// Source span `[start, end)` (in source texels, fractional) covered by destination texel `i`.
pub(super) fn span(i: u32, src: u32, dst: u32) -> (f32, f32) {
    let scale = src as f32 / dst as f32;
    (i as f32 * scale, (i + 1) as f32 * scale)
}

/// Visits the source texels of a span with their fractional weights (summing to the span width).
pub(super) fn for_each_weight(start: f32, end: f32, src: u32, mut f: impl FnMut(usize, f32)) {
    let first = start.floor() as u32;
    let last = (end.ceil() as u32).min(src);
    for s in first..last {
        let w = (end.min(s as f32 + 1.0) - start.max(s as f32)).max(0.0);
        if w > 0.0 {
            f(s as usize, w);
        }
    }
}

/// One premultiplied-linear RGBA texel: colour channels (weighted) and alpha.
pub(super) type Texel = [f32; 4];

/// Any size ratio: every destination texel averages the exact (fractional) source area it covers.
pub(super) fn downsample_general(src: &[u8], sw: u32, sh: u32, dw: u32, dh: u32, weighted: bool) -> Vec<u8> {
    let lut = srgb_to_linear_lut();
    // Horizontal pass: sw x sh -> dw x sh, in linear light. Channels 0..3 are colour (times the
    // alpha weight in coverage mode), channel 3 is alpha; the weight sum is kept per texel.
    let mut mid: Vec<(Texel, f32)> = Vec::with_capacity((dw * sh) as usize);
    for y in 0..sh as usize {
        let row = &src[y * sw as usize * 4..(y + 1) * sw as usize * 4];
        for x in 0..dw {
            let (a, b) = span(x, sw, dw);
            let mut acc = [0.0f32; 4];
            let mut wsum = 0.0;
            for_each_weight(a, b, sw, |s, w| {
                let p = &row[s * 4..s * 4 + 4];
                let alpha = f32::from(p[3]) / 255.0;
                let cw = if weighted { w * alpha } else { w };
                acc[0] += lut[p[0] as usize] * cw;
                acc[1] += lut[p[1] as usize] * cw;
                acc[2] += lut[p[2] as usize] * cw;
                acc[3] += alpha * w;
                wsum += cw;
            });
            mid.push((acc, wsum));
        }
    }
    // Vertical pass and encoding.
    let mut out = vec![0u8; (dw * dh * 4) as usize];
    for y in 0..dh {
        let (a, b) = span(y, sh, dh);
        for x in 0..dw as usize {
            let mut acc = [0.0f32; 4];
            let mut wsum = 0.0;
            let mut area = 0.0;
            for_each_weight(a, b, sh, |s, w| {
                let (t, ws) = &mid[s * dw as usize + x];
                for c in 0..4 {
                    acc[c] += t[c] * w;
                }
                wsum += ws * w;
                area += w;
            });
            let o = (y as usize * dw as usize + x) * 4;
            if wsum > 1e-9 {
                for c in 0..3 {
                    out[o + c] = encode_srgb(acc[c] / wsum);
                }
            } else {
                // No weight at all (fully transparent area in coverage mode): plain average.
                for c in 0..3 {
                    out[o + c] = encode_srgb(acc[c] / area.max(1e-9));
                }
            }
            let alpha_norm = acc[3] / (area * (sw as f32 / dw as f32)).max(1e-9);
            out[o + 3] = (alpha_norm.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
        }
    }
    out
}
