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
}

mod shape;

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
mod tests;
