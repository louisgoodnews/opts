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
    let mut count: HashMap<(Key, Key), (u8, Vec3, Vec3)> = HashMap::new();
    for tri in surfaces {
        for e in 0..3 {
            let (a, b) = (tri[e], tri[(e + 1) % 3]);
            let (ka, kb) = (key(a), key(b));
            if ka == kb {
                continue;
            }
            let k = if ka < kb { (ka, kb) } else { (kb, ka) };
            let entry = count.entry(k).or_insert((0, a, b));
            entry.0 = entry.0.saturating_add(1);
        }
    }
    count
        .into_values()
        .filter(|(n, ..)| *n == 1)
        .map(|(_, a, b)| (a, b))
        .collect()
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
mod tests {
    use super::*;
    use crate::terrain::tests::flat;
    use crate::terrain::{carve_terrain, TileTerrain};

    /// A straight strip from `(x0, 20)` to `(x1, 260)` at height `z`, as the two triangles per
    /// 40 m stretch of a spline ribbon.
    fn strip(x0: f32, x1: f32, z: f32) -> Vec<GroundTriangle> {
        let mut out = Vec::new();
        let mut y = 20.0;
        while y < 260.0 {
            let (a, b) = (Vec3::new(x0, y, z), Vec3::new(x1, y, z));
            let (c, d) = (Vec3::new(x0, y + 40.0, z), Vec3::new(x1, y + 40.0, z));
            out.push([a, b, c]);
            out.push([b, d, c]);
            y += 40.0;
        }
        out
    }

    fn at(t: &TileTerrain, x: usize, y: usize) -> f32 {
        t.grid.heights[y * 61 + x]
    }

    #[test]
    fn only_the_outer_edges_of_a_ribbon_are_boundary() {
        let edges = boundary_edges(&strip(41.0, 49.0, 0.3));
        // Two sides of six stretches each, the two ends.
        assert_eq!(edges.len(), 6 * 2 + 2);
        assert!(edges
            .iter()
            .all(|(a, b)| (a.x == b.x) || (a.y == b.y && a.y.abs() > 0.0)));
    }

    #[test]
    fn terrain_beside_a_surface_is_pulled_up_to_its_edge() {
        // The terrain lies 0.3 m under the surface; vertices 1 m outside its edges move up to it,
        // those 4 m outside (beyond the band) and those under it stay.
        let (carved, stats) = carve_terrain(&flat(0.0), &strip(41.0, 49.0, 0.3));
        assert!(stats.aligned > 0);
        let near = at(&carved, 8, 20); // x = 40, 1 m from the edge at 41
        assert!(near > 0.15 && near <= 0.27, "{near}");
        assert_eq!(at(&carved, 7, 20), 0.0, "x = 35 is outside the band");
        assert_eq!(
            at(&carved, 9, 20),
            0.0,
            "x = 45 is under the surface, below it already"
        );
    }

    #[test]
    fn terrain_beside_a_surface_is_pulled_down_to_its_edge() {
        // 0.4 m above the surface: the carve lowers the vertices next to it under the surface,
        // the alignment brings the neighbours of those down towards the edge too.
        let (carved, _) = carve_terrain(&flat(0.7), &strip(41.0, 49.0, 0.3));
        let near = at(&carved, 8, 20);
        assert!(near <= 0.27 + 1e-4, "never above the surface: {near}");
        assert_eq!(at(&carved, 7, 20), 0.7, "beyond the band");
    }

    #[test]
    fn a_bigger_gap_is_content_and_stays() {
        // A bridge deck 3 m above the ground: the terrain beside it is not raised to it.
        let (carved, stats) = carve_terrain(&flat(0.0), &strip(41.0, 49.0, 3.0));
        assert_eq!(stats.aligned, 0);
        assert_eq!(at(&carved, 8, 20), 0.0);
    }

    #[test]
    fn the_alignment_never_lifts_terrain_above_a_surface_it_overlaps() {
        // A slanted ribbon, low at the south end: a vertex beside it must not rise above what
        // the carve allows for the triangles that reach under the ribbon.
        let tris: Vec<GroundTriangle> = vec![
            [
                Vec3::new(41.0, 20.0, 0.1),
                Vec3::new(49.0, 20.0, 0.1),
                Vec3::new(41.0, 260.0, 0.6),
            ],
            [
                Vec3::new(49.0, 20.0, 0.1),
                Vec3::new(49.0, 260.0, 0.6),
                Vec3::new(41.0, 260.0, 0.6),
            ],
        ];
        let (carved, _) = carve_terrain(&flat(0.0), &tris);
        for y in 4..=52 {
            let ribbon = 0.1 + 0.5 * (y as f32 * 5.0 - 20.0) / 240.0;
            let v = at(&carved, 8, y);
            assert!(
                v <= ribbon + 0.04,
                "y {y}: terrain {v} over ribbon {ribbon}"
            );
        }
    }
}
