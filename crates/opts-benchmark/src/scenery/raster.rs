//! Binary image masks and a CPU triangle rasteriser for the expected silhouettes.
//!
//! A pixel is covered when its centre lies inside the projected triangle (both windings count:
//! the expected silhouette ignores back-face culling). Degenerate (edge-on) triangles cover
//! nothing, as on the GPU.

use bevy::math::Vec2;

/// A binary mask, row-major, row 0 at the top.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mask {
    /// Width in pixels.
    pub width: usize,
    /// Height in pixels.
    pub height: usize,
    /// Coverage per pixel.
    pub px: Vec<bool>,
}

impl Mask {
    /// An empty mask.
    #[must_use]
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            px: vec![false; width * height],
        }
    }

    /// Number of covered pixels.
    #[must_use]
    pub fn count(&self) -> usize {
        self.px.iter().filter(|&&p| p).count()
    }

    /// Whether pixel `(x, y)` is covered.
    #[must_use]
    pub fn get(&self, x: usize, y: usize) -> bool {
        self.px[y * self.width + x]
    }

    /// Pixels covered in both masks.
    #[must_use]
    pub fn and_count(&self, other: &Mask) -> usize {
        self.px
            .iter()
            .zip(&other.px)
            .filter(|(a, b)| **a && **b)
            .count()
    }

    /// Pixels covered in either mask.
    #[must_use]
    pub fn or_count(&self, other: &Mask) -> usize {
        self.px
            .iter()
            .zip(&other.px)
            .filter(|(a, b)| **a || **b)
            .count()
    }

    /// Adds every pixel of `other`.
    pub fn union_with(&mut self, other: &Mask) {
        for (a, b) in self.px.iter_mut().zip(&other.px) {
            *a |= *b;
        }
    }

    /// The pixels covered here but not in `other`.
    #[must_use]
    pub fn minus(&self, other: &Mask) -> Mask {
        Mask {
            width: self.width,
            height: self.height,
            px: self
                .px
                .iter()
                .zip(&other.px)
                .map(|(a, b)| *a && !*b)
                .collect(),
        }
    }

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

/// Fills every pixel whose centre lies inside the triangle `t` (pixel coordinates).
pub fn fill_triangle(mask: &mut Mask, t: [Vec2; 3]) {
    let edge = |a: Vec2, b: Vec2, p: Vec2| (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
    let area = edge(t[0], t[1], t[2]);
    if area.abs() < 1e-9 || t.iter().any(|v| !v.is_finite()) {
        return;
    }
    let sign = area.signum();
    let min = t[0].min(t[1]).min(t[2]);
    let max = t[0].max(t[1]).max(t[2]);
    let x0 = (min.x - 0.5).ceil().max(0.0) as usize;
    let y0 = (min.y - 0.5).ceil().max(0.0) as usize;
    let x1 = ((max.x - 0.5).floor()).min(mask.width as f32 - 1.0);
    let y1 = ((max.y - 0.5).floor()).min(mask.height as f32 - 1.0);
    if x1 < 0.0 || y1 < 0.0 {
        return;
    }
    let (x1, y1) = (x1 as usize, y1 as usize);
    for y in y0..=y1 {
        for x in x0..=x1 {
            let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            let inside = (0..3).all(|i| sign * edge(t[i], t[(i + 1) % 3], p) >= 0.0);
            if inside {
                mask.px[y * mask.width + x] = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_of_two_triangles_covers_its_area() {
        let mut m = Mask::new(16, 16);
        let (a, b, c, d) = (
            Vec2::new(2.0, 2.0),
            Vec2::new(10.0, 2.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(2.0, 10.0),
        );
        fill_triangle(&mut m, [a, b, c]);
        fill_triangle(&mut m, [a, d, c]); // opposite winding
        assert_eq!(m.count(), 64);
        assert_eq!(m.bbox(), Some([2, 2, 9, 9]));
        assert_eq!(m.centroid(), Some(Vec2::new(6.0, 6.0)));
    }

    #[test]
    fn dilation_grows_by_k() {
        let mut m = Mask::new(9, 9);
        m.px[4 * 9 + 4] = true;
        assert_eq!(m.dilate(2).count(), 25);
    }

    #[test]
    fn degenerate_and_offscreen_triangles_cover_nothing() {
        let mut m = Mask::new(8, 8);
        fill_triangle(
            &mut m,
            [
                Vec2::new(1.0, 1.0),
                Vec2::new(5.0, 5.0),
                Vec2::new(3.0, 3.0),
            ],
        );
        fill_triangle(
            &mut m,
            [
                Vec2::new(-10.0, -10.0),
                Vec2::new(-5.0, -10.0),
                Vec2::new(-5.0, -5.0),
            ],
        );
        assert_eq!(m.count(), 0);
    }
}
