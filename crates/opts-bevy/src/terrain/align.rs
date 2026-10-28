//! Aligning the terrain to the outer edges of the drawn ground surfaces.
//!
//! [`carve_terrain`](super::carve_terrain) only lowers the terrain that would cover a surface.
//! The terrain next to a road, a pavement or a parking bay is left as authored, which can be a
//! step of tens of centimetres below or above the surface's outer edge (the editor bakes the
//! terrain a hand's breadth under the road and leaves it alone beyond the first vertices), so
//! cars in a bay and objects beside the road float or sink. This module pulls the terrain
//! vertices that lie just outside a surface towards the height of its nearest **outer edge**:
//!
//! * the outer edges are the edges used by one surface triangle only ([`boundary_edges`]);
//! * a vertex not under any surface ([`covered`](super::carve_terrain)) within
//!   [`align_band`] of an edge moves towards the edge's height (minus [`CARVE_EPSILON_M`] so it
//!   never fights the surface) by a smooth weight that is 1 at the edge and 0 at the band;
//! * the move up or down is at most [`ALIGN_MAX_M`]: a bigger gap is content (an embankment, a
//!   bridge, a cutting) and stays as authored;
//! * a vertex never rises above its carve cap, so the terrain still never covers a surface.

use std::collections::HashMap;

use bevy::prelude::*;

use super::{GroundTriangle, CARVE_EPSILON_M};

/// Largest height change of one vertex by the alignment (m).
pub const ALIGN_MAX_M: f32 = 0.75;

/// Distance from an outer edge within which terrain vertices are aligned (m): a little over half
/// a terrain cell, so every cell next to an edge has an aligned vertex.
#[must_use]
pub fn align_band(cell: f32) -> f32 {
    (cell * 0.6).clamp(1.5, 4.0)
}

type Key = (i32, i32, i32);

fn key(v: Vec3) -> Key {
    (
        (v.x * 100.0).round() as i32,
        (v.y * 100.0).round() as i32,
        (v.z * 100.0).round() as i32,
    )
}

/// The edges of `surfaces` that only one triangle uses (corners compared to a centimetre).
#[must_use]
pub fn boundary_edges(surfaces: &[GroundTriangle]) -> Vec<(Vec3, Vec3)> {
    boundary_edges_out(surfaces)
        .into_iter()
        .map(|(a, b, _)| (a, b))
        .collect()
}

/// The outer edges with the unit direction that points away from their triangle (outside).
#[must_use]
pub fn boundary_edges_out(surfaces: &[GroundTriangle]) -> Vec<(Vec3, Vec3, Vec2)> {
    let mut count: HashMap<(Key, Key), (u8, Vec3, Vec3, Vec3)> = HashMap::new();
    for tri in surfaces {
        for e in 0..3 {
            let (a, b, c) = (tri[e], tri[(e + 1) % 3], tri[(e + 2) % 3]);
            let (ka, kb) = (key(a), key(b));
            if ka == kb {
                continue;
            }
            let k = if ka < kb { (ka, kb) } else { (kb, ka) };
            let entry = count.entry(k).or_insert((0, a, b, c));
            entry.0 = entry.0.saturating_add(1);
        }
    }
    count
        .into_values()
        .filter(|(n, ..)| *n == 1)
        .filter_map(|(_, a, b, c)| {
            let ab = (b - a).truncate();
            let len = ab.length();
            if len < 1e-4 {
                return None;
            }
            let mut out = Vec2::new(-ab.y, ab.x) / len;
            if out.dot((c - a).truncate()) > 0.0 {
                out = -out;
            }
            Some((a, b, out))
        })
        .collect()
}

/// How far outside an edge the fit looks (m): close enough that the first terrain triangle
/// beyond the surface is at the edge's height.
const FIT_OFFSET_M: f32 = 0.4;
/// Passes of the fit.
const FIT_PASSES: usize = 8;

/// The three vertices of the drawn terrain triangle at tile-local `(x, y)` and their weights.
fn triangle_at(n: usize, cell: f32, x: f32, y: f32) -> Option<([usize; 3], [f32; 3])> {
    let (gx, gy) = (x / cell, y / cell);
    if !(0.0..n as f32).contains(&gx) || !(0.0..n as f32).contains(&gy) {
        return None;
    }
    let dim = n + 1;
    let (i, j) = (gx as usize, gy as usize);
    let (u, w) = (gx - i as f32, gy - j as f32);
    let at = |i: usize, j: usize| j * dim + i;
    Some(if u + w <= 1.0 {
        ([at(i, j), at(i + 1, j), at(i, j + 1)], [1.0 - u - w, u, w])
    } else {
        (
            [at(i + 1, j + 1), at(i, j + 1), at(i + 1, j)],
            [u + w - 1.0, 1.0 - u, 1.0 - w],
        )
    })
}

/// Fits the terrain just outside every outer edge to the edge's height: along each edge the
/// drawn terrain [`FIT_OFFSET_M`] outside must be at the edge's height (minus
/// [`CARVE_EPSILON_M`]), whatever the 5 m terrain grid does there. The three vertices of the
/// terrain triangle at that point share the correction (the least change that fixes it), a few
/// passes over all edges settle neighbouring constraints; a vertex stays under its carve cap
/// (the terrain never covers a surface), within [`ALIGN_MAX_M`] of the authored height and
/// untouched when it is buried content. A residual bigger than [`ALIGN_MAX_M`] is content (a
/// bridge, a cutting): it is left alone. Returns how many vertices ended up changed by the fit.
pub(super) fn fit_edges(
    edges: &[(Vec3, Vec3, Vec2)],
    heights: &mut [f32],
    raw: &[f32],
    cap: &[f32],
    guarded: &[bool],
    n: usize,
    cell: f32,
) -> usize {
    if edges.is_empty() || n == 0 {
        return 0;
    }
    let before: Vec<f32> = heights.to_vec();
    for _ in 0..FIT_PASSES {
        for (a, b, out) in edges {
            let ab = (*b - *a).truncate();
            let len = ab.length();
            let dir = ab / len.max(1e-6);
            let mut s = 0.5_f32.min(len * 0.5);
            while s <= len {
                let q = a.truncate() + dir * s + *out * FIT_OFFSET_M;
                let target = a.z + (b.z - a.z) * (s / len.max(1e-6)) - CARVE_EPSILON_M;
                s += 1.0;
                let Some((v, w)) = triangle_at(n, cell, q.x, q.y) else {
                    continue;
                };
                let now: f32 = (0..3).map(|i| w[i] * heights[v[i]]).sum();
                let residual = target - now;
                if residual.abs() > ALIGN_MAX_M || residual.abs() < 0.005 {
                    continue;
                }
                let norm: f32 = w.iter().map(|x| x * x).sum();
                for i in 0..3 {
                    let k = v[i];
                    if guarded[k] {
                        continue;
                    }
                    let moved = heights[k] + residual * w[i] / norm * 0.7;
                    heights[k] = moved
                        .min(cap[k])
                        .clamp(raw[k] - ALIGN_MAX_M - 1.0, raw[k] + ALIGN_MAX_M);
                }
            }
        }
    }
    heights
        .iter()
        .zip(&before)
        .filter(|(a, b)| (**a - **b).abs() > 1e-4)
        .count()
}

/// Edges bucketed by position, each in every bucket its box expanded by the band touches.
struct EdgeIndex {
    edges: Vec<(Vec3, Vec3)>,
    buckets: HashMap<(i64, i64), Vec<u32>>,
    size: f32,
}

impl EdgeIndex {
    fn new(edges: Vec<(Vec3, Vec3)>, band: f32) -> Self {
        let size = band.max(0.5);
        let mut buckets: HashMap<(i64, i64), Vec<u32>> = HashMap::new();
        for (i, (a, b)) in edges.iter().enumerate() {
            let lo = a.truncate().min(b.truncate()) - Vec2::splat(band);
            let hi = a.truncate().max(b.truncate()) + Vec2::splat(band);
            for y in (lo.y / size).floor() as i64..=(hi.y / size).floor() as i64 {
                for x in (lo.x / size).floor() as i64..=(hi.x / size).floor() as i64 {
                    buckets.entry((x, y)).or_default().push(i as u32);
                }
            }
        }
        Self {
            edges,
            buckets,
            size,
        }
    }

    /// Distance in the plane to the nearest edge and that edge's height there.
    fn nearest(&self, p: Vec2) -> Option<(f32, f32)> {
        let cell = (
            (p.x / self.size).floor() as i64,
            (p.y / self.size).floor() as i64,
        );
        let mut best: Option<(f32, f32)> = None;
        for &i in self.buckets.get(&cell)? {
            let (a, b) = self.edges[i as usize];
            let (a2, b2) = (a.truncate(), b.truncate());
            let ab = b2 - a2;
            let len2 = ab.length_squared();
            let t = if len2 > 1e-10 {
                ((p - a2).dot(ab) / len2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let d = p.distance(a2 + ab * t);
            if best.is_none_or(|(bd, _)| d < bd) {
                best = Some((d, a.z + (b.z - a.z) * t));
            }
        }
        best
    }
}

/// Moves the uncovered vertices near an outer edge towards the edge's height (see the module
/// docs). `heights` is the grid after the carve (`(n + 1)²` values, row `b`, column `a`), `cap`
/// the carve caps, `covered` the vertices under a surface, `guarded` the vertices of buried
/// content. Returns how many vertices moved.
pub(super) fn align_vertices(
    edges: Vec<(Vec3, Vec3)>,
    heights: &mut [f32],
    cap: &[f32],
    covered: &[bool],
    guarded: &[bool],
    n: usize,
    cell: f32,
) -> usize {
    if edges.is_empty() {
        return 0;
    }
    let dim = n + 1;
    let band = align_band(cell);
    let index = EdgeIndex::new(edges, band);
    let mut moved = 0;
    for b in 0..dim {
        for a in 0..dim {
            let k = b * dim + a;
            if covered[k] || guarded[k] {
                continue;
            }
            let p = Vec2::new(a as f32 * cell, b as f32 * cell);
            let Some((d, edge_h)) = index.nearest(p) else {
                continue;
            };
            if d >= band {
                continue;
            }
            let target = edge_h - CARVE_EPSILON_M;
            let gap = target - heights[k];
            if gap.abs() > ALIGN_MAX_M {
                continue;
            }
            let t = 1.0 - d / band;
            let weight = t * t * (3.0 - 2.0 * t);
            let new = (heights[k] + gap * weight).min(cap[k]);
            if (new - heights[k]).abs() > 1e-4 {
                heights[k] = new;
                moved += 1;
            }
        }
    }
    moved
}

#[cfg(test)]
mod tests;
