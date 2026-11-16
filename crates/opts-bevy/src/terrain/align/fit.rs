//! Fitting the terrain just outside every outer edge to the edge's height, over a few passes.

use super::*;

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
pub(in crate::terrain) fn fit_edges(
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
