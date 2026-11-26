//! A triangle of a `[surface]` object's plate in the session frame, with its height map (M-0146).

use std::sync::Arc;

use opts_bevy::bevy::prelude::*;
use opts_road_surf::SurfMap;

/// A triangle of a surface plate in the session frame (x east, y north) with its heights, texture
/// coordinates and height map.
#[derive(Debug, Clone)]
pub struct SurfTri {
    /// Corners (m).
    pub xy: [Vec2; 3],
    /// Heights of the corners (m).
    pub z: [f32; 3],
    /// Texture coordinates of the corners.
    pub uv: [Vec2; 3],
    /// The texture's height map.
    pub map: Arc<SurfMap>,
    /// A synthetic map is laid in metres: the length it repeats over (m).
    pub period: Option<f32>,
}

impl SurfTri {
    /// The surface height and the unevenness at `p`, when `p` lies on the triangle.
    #[must_use]
    pub fn at(&self, p: Vec2) -> Option<(f32, f32)> {
        let [a, b, c] = self.xy;
        let d = (b.y - c.y) * (a.x - c.x) + (c.x - b.x) * (a.y - c.y);
        if d.abs() < 1e-9 {
            return None;
        }
        let w0 = ((b.y - c.y) * (p.x - c.x) + (c.x - b.x) * (p.y - c.y)) / d;
        let w1 = ((c.y - a.y) * (p.x - c.x) + (a.x - c.x) * (p.y - c.y)) / d;
        let w2 = 1.0 - w0 - w1;
        let eps = -1e-4;
        if w0 < eps || w1 < eps || w2 < eps {
            return None;
        }
        let z = w0 * self.z[0] + w1 * self.z[1] + w2 * self.z[2];
        let (u, v) = match self.period {
            Some(period) => (p.x / period, p.y / period),
            None => {
                let uv = self.uv[0] * w0 + self.uv[1] * w1 + self.uv[2] * w2;
                (uv.x, uv.y)
            }
        };
        Some((z, self.map.height_at(u, v)))
    }

    /// The triangle's extent (min, max) in x and y.
    #[must_use]
    pub fn bounds(&self) -> (Vec2, Vec2) {
        let lo = self.xy[0].min(self.xy[1]).min(self.xy[2]);
        let hi = self.xy[0].max(self.xy[1]).max(self.xy[2]);
        (lo, hi)
    }
}
