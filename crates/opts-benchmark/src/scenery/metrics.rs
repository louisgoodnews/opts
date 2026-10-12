//! Two-colour matting, expected masks and the per-view metrics.

use bevy::math::Vec2;

use super::expected::{ExpectedTri, Placement};
use super::framing::{map_to_bevy, View, IMAGE_SIZE};
use super::raster::{fill_triangle, Mask};
use super::ViewMetrics;

/// Dilation radius of the precision and recall tests (pixels).
pub const DILATE_PX: usize = 3;

/// The object as separated from the two backgrounds.
#[derive(Debug, Clone)]
pub struct Matte {
    /// Pixels with coverage `α ≥ 0.5`.
    pub mask: Mask,
    /// Un-premultiplied colour `I_black / α` (black where `α = 0`).
    pub rgb: Vec<[u8; 3]>,
    /// Object pixels in the sentinel colour.
    pub sentinel: Mask,
}

/// Separates the object from the backgrounds: `d = max |I_white − I_black|`, `α = 1 − d / 255`.
#[must_use]
pub fn matte(black: &[u8], white: &[u8]) -> Matte {
    let n = (IMAGE_SIZE * IMAGE_SIZE) as usize;
    let size = IMAGE_SIZE as usize;
    let mut out = Matte {
        mask: Mask::new(size, size),
        rgb: vec![[0; 3]; n],
        sentinel: Mask::new(size, size),
    };
    for (i, (b, w)) in black
        .chunks_exact(4)
        .zip(white.chunks_exact(4))
        .enumerate()
        .take(n)
    {
        let d = (0..3).map(|c| b[c].abs_diff(w[c])).max().unwrap_or(0);
        let alpha = 1.0 - f32::from(d) / 255.0;
        if alpha < 0.5 {
            continue;
        }
        out.mask.px[i] = true;
        let rgb = [0, 1, 2].map(|c| (f32::from(b[c]) / alpha).round().min(255.0) as u8);
        out.rgb[i] = rgb;
        let [r, g, bl] = opts_bevy::texture_sentinel::SENTINEL_RGB;
        if alpha > 0.9
            && rgb[0].abs_diff(r) <= 3
            && rgb[1].abs_diff(g) <= 3
            && rgb[2].abs_diff(bl) <= 3
        {
            out.sentinel.px[i] = true;
        }
    }
    out
}

/// The expected masks of one view.
#[derive(Debug, Clone)]
pub struct ExpectedMasks {
    /// Every drawable triangle.
    pub all: Mask,
    /// Triangles of `E_base`.
    pub base: Mask,
    /// Triangles drawn without alpha test or blending.
    pub solid: Mask,
    /// Text-textured triangles.
    pub text: Mask,
}

/// Rasterises the expected triangles, placed by `placement`, in `view`.
#[must_use]
pub fn expected_masks(tris: &[ExpectedTri], placement: &Placement, view: &View) -> ExpectedMasks {
    let size = IMAGE_SIZE as usize;
    let mut m = ExpectedMasks {
        all: Mask::new(size, size),
        base: Mask::new(size, size),
        solid: Mask::new(size, size),
        text: Mask::new(size, size),
    };
    for t in tris {
        let projected = t.v.map(|v| view.project(map_to_bevy(placement.to_map(v))));
        let [Some(a), Some(b), Some(c)] = projected else {
            continue;
        };
        let p = [a, b, c];
        fill_triangle(&mut m.all, p);
        if t.flags.base {
            fill_triangle(&mut m.base, p);
        }
        if !t.flags.alpha {
            fill_triangle(&mut m.solid, p);
        }
        if t.flags.text {
            fill_triangle(&mut m.text, p);
        }
    }
    m
}

fn ratio(num: usize, den: usize) -> Option<f32> {
    (den > 0).then(|| num as f32 / den as f32)
}

/// Computes the metrics of one view (without `best_rotation`).
#[must_use]
pub fn view_metrics(r: &Matte, e: &ExpectedMasks) -> ViewMetrics {
    let px_rendered = r.mask.count();
    let px_expected = e.all.count();
    let e_dil = e.all.dilate(DILATE_PX);
    let r_dil = r.mask.dilate(DILATE_PX);
    let base_n = e.base.count();
    let mut m = ViewMetrics {
        px_rendered,
        px_expected,
        px_expected_base: base_n,
        precision: ratio(r.mask.and_count(&e_dil), px_rendered),
        recall: ratio(r_dil.and_count(&e.base), base_n),
        iou: ratio(r.mask.and_count(&e.all), r.mask.or_count(&e.all)),
        alpha_share: ratio(e.all.minus_count(&e.solid), px_expected).unwrap_or(0.0),
        sentinel_share: ratio(r.sentinel.count(), px_rendered).unwrap_or(0.0),
        ..ViewMetrics::default()
    };
    if let (Some(rb), Some(eb)) = (r.mask.bbox(), e.all.bbox()) {
        let ew = (eb[2] - eb[0] + 1) as f32;
        let eh = (eb[3] - eb[1] + 1) as f32;
        let diag = Vec2::new(ew, eh).length().max(1.0);
        let over = [
            eb[0] as f32 - rb[0] as f32,
            eb[1] as f32 - rb[1] as f32,
            rb[2] as f32 - eb[2] as f32,
            rb[3] as f32 - eb[3] as f32,
        ]
        .into_iter()
        .fold(0.0f32, f32::max);
        m.bbox_overshoot = Some(over / diag);
        let rw = (rb[2] - rb[0] + 1) as f32;
        let rh = (rb[3] - rb[1] + 1) as f32;
        m.size_ratio = Some([rw / ew, rh / eh]);
        if let (Some(rc), Some(ec)) = (r.mask.centroid(), e.all.centroid()) {
            m.centroid_offset = Some((rc - ec).length() / diag);
        }
    }
    m
}

/// Mean colour difference (share of full scale, mean over channels) between two renders inside
/// `region`, counting pixels where both renders show the object.
#[must_use]
pub fn mean_colour_diff(a: &Matte, b: &Matte, region: &Mask) -> Option<f32> {
    let (mut sum, mut n) = (0.0f64, 0usize);
    for i in 0..region.px.len() {
        if !(region.px[i] && a.mask.px[i] && b.mask.px[i]) {
            continue;
        }
        let d: u32 = (0..3)
            .map(|c| u32::from(a.rgb[i][c].abs_diff(b.rgb[i][c])))
            .sum();
        sum += f64::from(d) / (3.0 * 255.0);
        n += 1;
    }
    (n > 0).then(|| (sum / n as f64) as f32)
}

/// Share of the pixels in `region` that differ between two renders: a pixel counts when only
/// one render shows the object there, or when both do and a colour channel differs by at least
/// `delta` (0..255). Pixels where neither render shows the object are ignored (a transparent
/// board with different glyphs changes only where either set has ink). `Some(0.0)` when the
/// region is not empty but neither render shows anything in it; `None` for an empty region.
#[must_use]
pub fn changed_share(a: &Matte, b: &Matte, region: &Mask, delta: u8) -> Option<f32> {
    if region.count() == 0 {
        return None;
    }
    let (mut changed, mut n) = (0usize, 0usize);
    for i in 0..region.px.len() {
        let (ia, ib) = (a.mask.px[i], b.mask.px[i]);
        if !region.px[i] || !(ia || ib) {
            continue;
        }
        n += 1;
        let differs = ia != ib || (0..3).any(|c| a.rgb[i][c].abs_diff(b.rgb[i][c]) >= delta);
        if differs {
            changed += 1;
        }
    }
    Some(if n == 0 {
        0.0
    } else {
        changed as f32 / n as f32
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buffers(object: &[usize], colour: [u8; 3]) -> (Vec<u8>, Vec<u8>) {
        let n = (IMAGE_SIZE * IMAGE_SIZE) as usize;
        let (mut b, mut w) = (vec![0u8; n * 4], vec![255u8; n * 4]);
        for c in w.chunks_exact_mut(4) {
            c[3] = 255;
        }
        for &i in object {
            b[i * 4..i * 4 + 3].copy_from_slice(&colour);
            w[i * 4..i * 4 + 3].copy_from_slice(&colour);
        }
        (b, w)
    }

    #[test]
    fn matting_finds_object_pixels_of_any_colour() {
        for colour in [[0, 0, 0], [255, 255, 255], [255, 0, 255], [30, 200, 90]] {
            let (b, w) = buffers(&[10, 11, 600], colour);
            let m = matte(&b, &w);
            assert_eq!(m.mask.count(), 3);
            assert_eq!(m.rgb[10], colour);
            assert_eq!(
                m.sentinel.count(),
                if colour == [255, 0, 255] { 3 } else { 0 }
            );
        }
    }

    #[test]
    fn changed_share_sees_moved_glyphs_and_ignores_empty_background() {
        let region = Mask {
            width: 512,
            height: 512,
            px: vec![true; 512 * 512],
        };
        let (b1, w1) = buffers(&[10, 11], [0, 0, 0]);
        let (b2, w2) = buffers(&[12, 13], [0, 0, 0]);
        let (ma, mb) = (matte(&b1, &w1), matte(&b2, &w2));
        assert_eq!(changed_share(&ma, &mb, &region, 40), Some(1.0));
        assert_eq!(changed_share(&ma, &ma, &region, 40), Some(0.0));
        let (b0, w0) = buffers(&[], [0, 0, 0]);
        let blank = matte(&b0, &w0);
        assert_eq!(changed_share(&blank, &blank, &region, 40), Some(0.0));
        let empty = Mask::new(512, 512);
        assert_eq!(changed_share(&ma, &mb, &empty, 40), None);
    }

    #[test]
    fn identical_masks_give_perfect_metrics() {
        let (b, w) = buffers(&(1000..1100).collect::<Vec<_>>(), [90, 90, 90]);
        let r = matte(&b, &w);
        let e = ExpectedMasks {
            all: r.mask.clone(),
            base: r.mask.clone(),
            solid: r.mask.clone(),
            text: Mask::new(512, 512),
        };
        let m = view_metrics(&r, &e);
        assert_eq!(m.precision, Some(1.0));
        assert_eq!(m.recall, Some(1.0));
        assert_eq!(m.iou, Some(1.0));
        assert_eq!(m.bbox_overshoot, Some(0.0));
        assert_eq!(m.size_ratio, Some([1.0, 1.0]));
    }
}
