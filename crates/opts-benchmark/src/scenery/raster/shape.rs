//! Mask operations beyond the basic set: discs, dilation, bounding box and centroid.

use bevy::math::Vec2;

use super::Mask;

impl Mask {
    /// Covers the pixels within `radius` of `centre` (pixel coordinates).
    pub fn add_disc(&mut self, centre: Vec2, radius: f32) {
        let (w, h) = (self.width as i64, self.height as i64);
        let r = radius.ceil() as i64;
        let (cx, cy) = (centre.x.floor() as i64, centre.y.floor() as i64);
        for y in (cy - r).max(0)..=(cy + r).min(h - 1) {
            for x in (cx - r).max(0)..=(cx + r).min(w - 1) {
                let d = Vec2::new(x as f32 + 0.5, y as f32 + 0.5) - centre;
                if d.length() <= radius {
                    self.px[(y * w + x) as usize] = true;
                }
            }
        }
    }

    /// Pixels covered here but not in `other`.
    #[must_use]
    pub fn minus_count(&self, other: &Mask) -> usize {
        self.px
            .iter()
            .zip(&other.px)
            .filter(|(a, b)| **a && !**b)
            .count()
    }

    /// The mask dilated by `k` pixels (square neighbourhood, separable maximum filter).
    #[must_use]
    pub fn dilate(&self, k: usize) -> Mask {
        let (w, h) = (self.width, self.height);
        let mut rows = Mask::new(w, h);
        for y in 0..h {
            for x in 0..w {
                let lo = x.saturating_sub(k);
                let hi = (x + k).min(w - 1);
                rows.px[y * w + x] = (lo..=hi).any(|i| self.px[y * w + i]);
            }
        }
        let mut out = Mask::new(w, h);
        for y in 0..h {
            let lo = y.saturating_sub(k);
            let hi = (y + k).min(h - 1);
            for x in 0..w {
                out.px[y * w + x] = (lo..=hi).any(|j| rows.px[j * w + x]);
            }
        }
        out
    }

    /// Bounding box `(min_x, min_y, max_x, max_y)` of the covered pixels (inclusive).
    #[must_use]
    pub fn bbox(&self) -> Option<[usize; 4]> {
        let mut b: Option<[usize; 4]> = None;
        for y in 0..self.height {
            for x in 0..self.width {
                if !self.get(x, y) {
                    continue;
                }
                let bb = b.get_or_insert([x, y, x, y]);
                bb[0] = bb[0].min(x);
                bb[1] = bb[1].min(y);
                bb[2] = bb[2].max(x);
                bb[3] = bb[3].max(y);
            }
        }
        b
    }

    /// Mean position of the covered pixels (pixel centres).
    #[must_use]
    pub fn centroid(&self) -> Option<Vec2> {
        let (mut sum, mut n) = (Vec2::ZERO, 0usize);
        for y in 0..self.height {
            for x in 0..self.width {
                if self.get(x, y) {
                    sum += Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                    n += 1;
                }
            }
        }
        (n > 0).then(|| sum / n as f32)
    }
}
