//! CPU mip-chain generation for uncompressed colour textures (M-0058 phase B).
//!
//! About 83 % of the corpus textures ship without a mipmap chain, and a texture without one
//! aliases as soon as it covers fewer screen pixels than texels. [`generate_mip_chain`] appends
//! the missing levels to an `Rgba8UnormSrgb` [`Image`]:
//!
//! - **sRGB-correct**: texels are averaged in linear light (a black/white checker becomes 188,
//!   not 128);
//! - **any size**: level `n` has `max(1, size >> n)` texels per axis (the layout wgpu expects), and
//!   every texel averages the exact (fractional) source area it covers, so non-power-of-two
//!   textures need no padding;
//! - **alpha** is averaged linearly and left alone, unless a coverage cutoff is given
//!   ([`MipOptions::alpha_cutoff`]): then colours are weighted by alpha and each level's alpha is
//!   scaled so that the share of texels at or above the cutoff stays that of the base level
//!   (foliage and fences do not thin out with distance; used by M-0059).
//!
//! Block-compressed textures and 8-bit masks are not touched.

use std::sync::OnceLock;

use bevy::image::Image;
use bevy::render::render_resource::TextureFormat;

/// Options of [`generate_mip_chain`].
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MipOptions {
    /// Alpha-coverage mode: the alpha cutoff (`0..=1`) of the material the texture is used with.
    /// `None` leaves alpha as the plain average.
    pub alpha_cutoff: Option<f32>,
    /// Weights colours by alpha when averaging (always on in coverage mode), so transparent texels
    /// with black RGB do not darken the colour of the visible ones. For glyph and sign textures.
    pub weight_colour_by_alpha: bool,
}

/// Number of levels of a full chain for a texture of `width` x `height` texels.
#[must_use]
pub fn full_chain_levels(width: u32, height: u32) -> u32 {
    32 - width.max(height).max(1).leading_zeros()
}

/// Size of mip level `level` (never below 1).
#[must_use]
pub fn level_size(width: u32, height: u32, level: u32) -> (u32, u32) {
    ((width >> level).max(1), (height >> level).max(1))
}

fn srgb_to_linear_lut() -> &'static [f32; 256] {
    static LUT: OnceLock<[f32; 256]> = OnceLock::new();
    LUT.get_or_init(|| {
        let mut lut = [0.0; 256];
        for (i, v) in lut.iter_mut().enumerate() {
            let c = i as f32 / 255.0;
            *v = if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            };
        }
        lut
    })
}

const LINEAR_STEPS: usize = 4096;

fn linear_to_srgb_lut() -> &'static [u8; LINEAR_STEPS + 1] {
    static LUT: OnceLock<[u8; LINEAR_STEPS + 1]> = OnceLock::new();
    LUT.get_or_init(|| {
        let mut lut = [0u8; LINEAR_STEPS + 1];
        for (i, v) in lut.iter_mut().enumerate() {
            let l = i as f32 / LINEAR_STEPS as f32;
            let c = if l <= 0.003_130_8 {
                l * 12.92
            } else {
                1.055 * l.powf(1.0 / 2.4) - 0.055
            };
            *v = (c * 255.0 + 0.5).clamp(0.0, 255.0) as u8;
        }
        lut
    })
}

fn encode_srgb(linear: f32) -> u8 {
    let i = (linear.clamp(0.0, 1.0) * LINEAR_STEPS as f32 + 0.5) as usize;
    linear_to_srgb_lut()[i.min(LINEAR_STEPS)]
}

/// Source span `[start, end)` (in source texels, fractional) covered by destination texel `i`.
fn span(i: u32, src: u32, dst: u32) -> (f32, f32) {
    let scale = src as f32 / dst as f32;
    (i as f32 * scale, (i + 1) as f32 * scale)
}

/// Visits the source texels of a span with their fractional weights (summing to the span width).
fn for_each_weight(start: f32, end: f32, src: u32, mut f: impl FnMut(usize, f32)) {
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
type Texel = [f32; 4];

const ENC_STEPS: usize = 16384;

/// sRGB byte -> linear light as 16-bit integer (for the integer fast path).
fn srgb_to_linear16() -> &'static [u16; 256] {
    static LUT: OnceLock<[u16; 256]> = OnceLock::new();
    LUT.get_or_init(|| {
        let f = srgb_to_linear_lut();
        let mut lut = [0u16; 256];
        for (o, v) in lut.iter_mut().zip(f) {
            *o = (v * 65535.0 + 0.5) as u16;
        }
        lut
    })
}

/// 14-bit linear light -> sRGB byte.
fn linear14_to_srgb() -> &'static [u8; ENC_STEPS + 1] {
    static LUT: OnceLock<[u8; ENC_STEPS + 1]> = OnceLock::new();
    LUT.get_or_init(|| {
        let mut lut = [0u8; ENC_STEPS + 1];
        for (i, v) in lut.iter_mut().enumerate() {
            let l = i as f32 / ENC_STEPS as f32;
            let c = if l <= 0.003_130_8 {
                l * 12.92
            } else {
                1.055 * l.powf(1.0 / 2.4) - 0.055
            };
            *v = (c * 255.0 + 0.5).clamp(0.0, 255.0) as u8;
        }
        lut
    })
}

/// Integer fast path of the most common step: plain 2x2 mean of a level whose width and height
/// both halve exactly (colour averaged in linear light, alpha as the plain mean).
fn downsample_2x2_plain(src: &[u8], sw: usize, dw: usize, dh: usize) -> Vec<u8> {
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
fn downsample_exact(
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
fn downsample(src: &[u8], sw: u32, sh: u32, dw: u32, dh: u32, weighted: bool) -> Vec<u8> {
    let exact = |s: u32, d: u32| (s == d || s == 2 * d).then_some((s / d) as usize);
    if let (Some(kx), Some(ky)) = (exact(sw, dw), exact(sh, dh)) {
        if !weighted && kx == 2 && ky == 2 {
            return downsample_2x2_plain(src, sw as usize, dw as usize, dh as usize);
        }
        return downsample_exact(src, sw, dw, dh, (kx, ky), weighted);
    }
    downsample_general(src, sw, sh, dw, dh, weighted)
}

/// Any size ratio: every destination texel averages the exact (fractional) source area it covers.
fn downsample_general(src: &[u8], sw: u32, sh: u32, dw: u32, dh: u32, weighted: bool) -> Vec<u8> {
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

fn coverage(level: &[u8], cutoff: f32, scale: f32) -> f32 {
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
fn match_coverage(level: &mut [u8], cutoff: f32, target: f32) {
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

/// Running totals of the chains generated in this process (for load-time reports).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MipStats {
    /// Textures that received a generated chain.
    pub textures: u64,
    /// Time spent generating, in microseconds.
    pub micros: u64,
}

static GENERATED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static MICROS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// The totals so far.
#[must_use]
pub fn mip_stats() -> MipStats {
    use std::sync::atomic::Ordering::Relaxed;
    MipStats {
        textures: GENERATED.load(Relaxed),
        micros: MICROS.load(Relaxed),
    }
}

/// Gives an uncompressed `Rgba8UnormSrgb` image a full mip chain. Returns `true` when levels were
/// added; images that already have a chain, are block-compressed, masks (`R8`) or have
/// inconsistent data are left untouched (`false`).
pub fn generate_mip_chain(image: &mut Image, options: MipOptions) -> bool {
    let desc = &image.texture_descriptor;
    if desc.mip_level_count > 1
        || desc.format != TextureFormat::Rgba8UnormSrgb
        || desc.size.depth_or_array_layers != 1
        || desc.size.width.max(desc.size.height) < 2
    {
        return false;
    }
    let (w, h) = (desc.size.width, desc.size.height);
    let started = std::time::Instant::now();
    let Some(chain) = image
        .data
        .as_deref()
        .and_then(|base| rgba8_srgb_chain(base, w, h, options))
    else {
        return false;
    };
    image.data = Some(chain);
    image.texture_descriptor.mip_level_count = full_chain_levels(w, h);
    use std::sync::atomic::Ordering::Relaxed;
    GENERATED.fetch_add(1, Relaxed);
    MICROS.fetch_add(started.elapsed().as_micros() as u64, Relaxed);
    true
}

#[cfg(test)]
mod tests {
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
}
