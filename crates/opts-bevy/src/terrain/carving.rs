//! The carving state of a tile: the cut-out regions of the road and spline surfaces.

use super::*;

/// What the carve learns from the surface triangles, per grid vertex: the height it must stay
/// below (`cap`), whether the terrain is far above and left alone (`guarded`) and whether a
/// surface lies over it (`covered`).
pub(super) struct Carving {
    pub(super) cap: Vec<f32>,
    pub(super) guarded: Vec<bool>,
    pub(super) covered: Vec<bool>,
    pub(super) n: usize,
    pub(super) dim: usize,
    pub(super) cell: f32,
}

impl Carving {
    pub(super) fn new(n: usize, cell: f32) -> Self {
        let dim = n + 1;
        Self {
            cap: vec![f32::INFINITY; dim * dim],
            guarded: vec![false; dim * dim],
            covered: vec![false; dim * dim],
            n,
            dim,
            cell,
        }
    }

    /// Takes one surface triangle into account: marks the vertices under it and caps the ones its
    /// overlap with the terrain's cells reaches.
    pub(super) fn add(&mut self, tri: &GroundTriangle, raw: &[f32]) {
        let (n, dim, cell) = (self.n, self.dim, self.cell);
        let ring = -1_i64..=n as i64;
        let p = tri.map(|v| v.truncate());
        let area = cross2(p[0], p[1], p[2]);
        if area.abs() < 1e-8 {
            return;
        }
        // Counter-clockwise for the clipper; the height plane follows the vertices.
        let (p, h) = if area > 0.0 {
            (p, [tri[0].z, tri[1].z, tri[2].z])
        } else {
            ([p[0], p[2], p[1]], [tri[0].z, tri[2].z, tri[1].z])
        };
        let height_at = |x: Vec2| {
            let a = cross2(p[0], p[1], p[2]);
            let l0 = cross2(x, p[1], p[2]) / a;
            let l1 = cross2(p[0], x, p[2]) / a;
            l0 * h[0] + l1 * h[1] + (1.0 - l0 - l1) * h[2]
        };
        let lo = p[0].min(p[1]).min(p[2]) / cell;
        let hi = p[0].max(p[1]).max(p[2]) / cell;
        let i0 = (lo.x.floor() as i64).max(*ring.start());
        let i1 = (hi.x.floor() as i64).min(*ring.end());
        let j0 = (lo.y.floor() as i64).max(*ring.start());
        let j1 = (hi.y.floor() as i64).min(*ring.end());
        // The vertices under this surface triangle are covered: the alignment leaves them to the
        // carve.
        for j in j0.max(0)..=(j1 + 1).min(n as i64) {
            for i in i0.max(0)..=(i1 + 1).min(n as i64) {
                let x = Vec2::new(i as f32 * cell, j as f32 * cell);
                if (0..3).all(|e| cross2(p[e], p[(e + 1) % 3], x) >= 0.0) {
                    self.covered[j as usize * dim + i as usize] = true;
                }
            }
        }
        for j in j0..=j1 {
            for i in i0..=i1 {
                let v = |di: i64, dj: i64| (i + di, j + dj);
                for corners in [[v(0, 0), v(1, 0), v(0, 1)], [v(1, 0), v(1, 1), v(0, 1)]] {
                    let pts = corners.map(|(a, b)| Vec2::new(a as f32 * cell, b as f32 * cell));
                    let overlap = clip_to_triangle(&pts, p);
                    if overlap.len() < 3 {
                        continue;
                    }
                    let overlap_area: f32 = (1..overlap.len() - 1)
                        .map(|k| cross2(overlap[0], overlap[k], overlap[k + 1]))
                        .sum();
                    if overlap_area.abs() < 1e-6 {
                        continue;
                    }
                    let m = overlap
                        .iter()
                        .map(|&x| height_at(x))
                        .fold(f32::INFINITY, f32::min)
                        - CARVE_EPSILON_M;
                    for (a, b) in corners {
                        if !(0..=n as i64).contains(&a) || !(0..=n as i64).contains(&b) {
                            continue;
                        }
                        let k = b as usize * dim + a as usize;
                        if raw[k] - m > CARVE_COVER_M {
                            self.guarded[k] = true;
                        } else if m < self.cap[k] {
                            self.cap[k] = m;
                        }
                    }
                }
            }
        }
    }
}
