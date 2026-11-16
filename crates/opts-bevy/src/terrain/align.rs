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
mod edges;
mod fit;
mod index;

pub use edges::{boundary_edges, boundary_edges_out};
pub(super) use fit::fit_edges;
use index::EdgeIndex;

/// Largest height change of one vertex by the alignment (m).
pub const ALIGN_MAX_M: f32 = 0.75;

/// Distance from an outer edge within which terrain vertices are aligned (m): a little over half
/// a terrain cell, so every cell next to an edge has an aligned vertex.
#[must_use]
pub fn align_band(cell: f32) -> f32 {
    (cell * 0.6).clamp(1.5, 4.0)
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
