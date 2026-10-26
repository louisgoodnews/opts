//! Terrain shaped to aligned splines (`docs/src/architecture/terrain-carving.md` §10).
//!
//! A spline marked `[spline_terrain_align]` / `[spline_terrain_align_2]` has the terrain under
//! its `[heightprofile]` strips set to their height, however far that is from the authored grid:
//! OMSI's own terrain cache (`*.map.terrain_0.rdy`) holds the result, a re-meshed terrain whose
//! vertices follow the strips' edges (Winsenburg tile -3/-18: the grid under a road 12 m down in a
//! trough is gone, the terrain lies at the road's -11.9 m). The carve only ever moves grid
//! vertices by at most a metre, so such a road stayed under the terrain.
//!
//! [`TerrainCut`] holds the strips' triangles. [`build_cut_terrain_mesh`] clips every terrain
//! triangle against them: the part inside a strip takes the strip's height, the part outside
//! keeps the grid's, and a steep face along the strips' outer edges joins the two. The height
//! function ([`super::TileTerrain::height`]) answers from the strips first.

use std::collections::HashMap;

use super::{
    boundary_edges_out, clip_to_triangle, cross2, terrain_height, GroundTriangle,
    MapSplineInstance, MapTerrain, SplineProfile, CARVE_EPSILON_M,
};
use crate::mesh::smooth_normal_mesh;
use crate::scenery::spline_stations;
use bevy::prelude::*;

/// How far the terrain stays under an aligned strip (m): it never covers the road on it.
const CUT_EPSILON_M: f32 = 0.03;
/// A height step along a strip edge below this (m) needs no joining face.
const STEP_MIN_M: f32 = 0.02;
/// How far beyond an outer edge another strip is looked for (m): a strip that continues there
/// makes the edge no step (the two halves of a road, a spline's next segment).
const EDGE_PROBE_M: f32 = 0.05;

/// The `[heightprofile]` strips of an aligned spline, extruded over the drawn spline's stations
/// (as [`super::spline_ground_triangles`] does for the drawn surface). `offset` is the corner of
/// the spline's tile in the wanted frame. Profiles without a height profile give nothing.
#[must_use]
pub fn spline_cut_triangles(
    profile: &SplineProfile,
    inst: &MapSplineInstance,
    offset: (f32, f32),
) -> Vec<GroundTriangle> {
    if profile.height_profiles.is_empty() {
        return Vec::new();
    }
    let side = if inst.mirrored { -1.0 } else { 1.0 };
    let stations = spline_stations(inst, offset);
    let at = |k: usize, x: f32, z: f32| {
        let (ex, ny, sz, h, _) = stations[k];
        let x = x * side;
        Vec3::new(ex + h.cos() * x, ny - h.sin() * x, sz + z - CUT_EPSILON_M)
    };
    let mut out = Vec::new();
    for k in 0..stations.len().saturating_sub(1) {
        for hp in &profile.height_profiles {
            let (a, b) = (at(k, hp.x_start, hp.z_start), at(k, hp.x_end, hp.z_end));
            let (c, d) = (
                at(k + 1, hp.x_start, hp.z_start),
                at(k + 1, hp.x_end, hp.z_end),
            );
            out.push([a, b, c]);
            out.push([b, d, c]);
        }
    }
    out
}

/// Triangles indexed by the terrain cells their bounding boxes touch.
#[derive(Debug, Clone, PartialEq, Default)]
struct TriIndex {
    /// Counter-clockwise (in east/north) triangles.
    tris: Vec<GroundTriangle>,
    cell: f32,
    /// Cell `(i, j)` to the triangles that touch it, in triangle order.
    cells: HashMap<(i32, i32), Vec<u32>>,
}

impl TriIndex {
    /// Indexes `tris` (any winding; degenerate ones are dropped) over cells of `cell` metres.
    fn new(tris: Vec<GroundTriangle>, cell: f32) -> Self {
        let tris: Vec<GroundTriangle> = tris
            .into_iter()
            .filter_map(|t| {
                let area = cross2(t[0].truncate(), t[1].truncate(), t[2].truncate());
                if area.abs() < 1e-6 {
                    None
                } else if area < 0.0 {
                    Some([t[0], t[2], t[1]])
                } else {
                    Some(t)
                }
            })
            .collect();
        let mut cells: HashMap<(i32, i32), Vec<u32>> = HashMap::new();
        for (n, t) in tris.iter().enumerate() {
            let lo = t[0].min(t[1]).min(t[2]);
            let hi = t[0].max(t[1]).max(t[2]);
            let (i0, j0) = ((lo.x / cell).floor() as i32, (lo.y / cell).floor() as i32);
            let (i1, j1) = ((hi.x / cell).floor() as i32, (hi.y / cell).floor() as i32);
            for j in j0..=j1 {
                for i in i0..=i1 {
                    cells.entry((i, j)).or_default().push(n as u32);
                }
            }
        }
        Self { tris, cell, cells }
    }

    fn candidates(&self, i: i32, j: i32) -> &[u32] {
        self.cells.get(&(i, j)).map_or(&[], Vec::as_slice)
    }

    /// The heights at `(x, y)` of the triangles that cover it, in triangle order.
    fn heights_at(&self, x: f32, y: f32) -> impl Iterator<Item = f32> + '_ {
        let key = (
            (x / self.cell).floor() as i32,
            (y / self.cell).floor() as i32,
        );
        let p = Vec2::new(x, y);
        self.candidates(key.0, key.1).iter().filter_map(move |&n| {
            let t = &self.tris[n as usize];
            let (a, b, c) = (t[0].truncate(), t[1].truncate(), t[2].truncate());
            let tol = -1e-4;
            (cross2(a, b, p) >= tol && cross2(b, c, p) >= tol && cross2(c, a, p) >= tol)
                .then(|| plane_height(t, p))
        })
    }
}

/// The aligned strips over one tile, in its frame, and the drawn ground they must stay under.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TerrainCut {
    strips: TriIndex,
    /// The drawn ground (surface ribbons, crossing faces): a strip is coarser than the road on
    /// it (one level across a kerbed street), so the terrain takes the lower of the two.
    ground: TriIndex,
    /// The outer edges that are steps: no other strip continues beyond them.
    edges: Vec<(Vec3, Vec3)>,
}

impl TerrainCut {
    /// Indexes the aligned `strips` and the drawn `ground` over cells of `cell` metres.
    #[must_use]
    pub fn new(strips: Vec<GroundTriangle>, ground: Vec<GroundTriangle>, cell: f32) -> Self {
        let strips = TriIndex::new(strips, cell);
        let ground = if strips.tris.is_empty() {
            TriIndex::default()
        } else {
            TriIndex::new(ground, cell)
        };
        let mut cut = Self {
            strips,
            ground,
            edges: Vec::new(),
        };
        cut.edges = boundary_edges_out(&cut.strips.tris)
            .into_iter()
            .filter(|(a, b, out)| {
                let mid = ((*a + *b) * 0.5).truncate() + *out * EDGE_PROBE_M;
                cut.height(mid.x, mid.y).is_none()
            })
            .map(|(a, b, _)| (a, b))
            .collect();
        cut
    }

    /// Whether there is no strip.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.strips.tris.is_empty()
    }

    /// Number of strip triangles.
    #[must_use]
    pub fn len(&self) -> usize {
        self.strips.tris.len()
    }

    /// The terrain's height at `(x, y)` when a strip covers it: the first such strip's, at most
    /// the lowest drawn ground there less the carve's clearance.
    #[must_use]
    pub fn height(&self, x: f32, y: f32) -> Option<f32> {
        let h = self.strips.heights_at(x, y).next()?;
        Some(self.capped(h, x, y))
    }

    /// `h` kept [`CARVE_EPSILON_M`] under the lowest drawn ground at `(x, y)`.
    fn capped(&self, h: f32, x: f32, y: f32) -> f32 {
        self.ground
            .heights_at(x, y)
            .fold(h, |h, g| h.min(g - CARVE_EPSILON_M))
    }

    fn candidates(&self, i: i32, j: i32) -> &[u32] {
        self.strips.candidates(i, j)
    }
}

/// Height of the plane through `t` at `p`.
fn plane_height(t: &GroundTriangle, p: Vec2) -> f32 {
    let (a, b, c) = (t[0].truncate(), t[1].truncate(), t[2].truncate());
    let area = cross2(a, b, c);
    if area.abs() < 1e-9 {
        return t[0].z;
    }
    let wa = cross2(b, c, p) / area;
    let wb = cross2(c, a, p) / area;
    t[0].z * wa + t[1].z * wb + t[2].z * (1.0 - wa - wb)
}

/// The part of convex `poly` on the right of the directed line `p → q` (outside a
/// counter-clockwise triangle edge), and the part on its left.
fn split(poly: &[Vec2], p: Vec2, q: Vec2) -> (Vec<Vec2>, Vec<Vec2>) {
    let (mut right, mut left) = (Vec::new(), Vec::new());
    for k in 0..poly.len() {
        let (a, b) = (poly[k], poly[(k + 1) % poly.len()]);
        let (sa, sb) = (cross2(p, q, a), cross2(p, q, b));
        if sa >= 0.0 {
            left.push(a);
        } else {
            right.push(a);
        }
        if (sa >= 0.0) != (sb >= 0.0) {
            let x = a + (b - a) * (sa / (sa - sb));
            left.push(x);
            right.push(x);
        }
    }
    (right, left)
}

/// Area of a polygon (counter-clockwise positive).
fn area(poly: &[Vec2]) -> f32 {
    (1..poly.len().saturating_sub(1))
        .map(|k| cross2(poly[0], poly[k], poly[k + 1]))
        .sum::<f32>()
        * 0.5
}

/// Mesh under construction: vertices in the tile frame `(east, north, height)`.
struct Builder {
    origin: (f32, f32),
    span: f32,
    cell: f32,
    uv_per_tile: bool,
    positions: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

impl Builder {
    fn vertex(&mut self, x: f32, y: f32, h: f32) -> u32 {
        self.positions
            .push([self.origin.0 + x, h, -(self.origin.1 + y)]);
        self.uvs.push(if self.uv_per_tile {
            [x / self.span, 1.0 - y / self.span]
        } else {
            [x / self.cell, y / self.cell]
        });
        (self.positions.len() - 1) as u32
    }

    /// A counter-clockwise polygon as a fan, heights from `height`.
    fn polygon(&mut self, poly: &[Vec2], height: impl Fn(Vec2) -> f32) {
        if poly.len() < 3 || area(poly) < 1e-5 {
            return;
        }
        let first = self.positions.len() as u32;
        for p in poly {
            self.vertex(p.x, p.y, height(*p));
        }
        for k in 1..poly.len() as u32 - 1 {
            self.indices
                .extend_from_slice(&[first, first + k, first + k + 1]);
        }
    }

    /// A vertical face from `top` to `bottom` along `a → b`, seen from both sides.
    fn face(&mut self, a: Vec2, b: Vec2, top: (f32, f32), bottom: (f32, f32)) {
        let v = [
            self.vertex(a.x, a.y, top.0),
            self.vertex(b.x, b.y, top.1),
            self.vertex(b.x, b.y, bottom.1),
            self.vertex(a.x, a.y, bottom.0),
        ];
        self.indices
            .extend_from_slice(&[v[0], v[1], v[2], v[0], v[2], v[3]]);
        self.indices
            .extend_from_slice(&[v[0], v[2], v[1], v[0], v[3], v[2]]);
    }
}

/// The terrain mesh of a tile shaped to `cut` (see the module docs), in absolute Bevy world
/// coordinates like [`super::build_terrain_mesh`], whose grid split and UVs it keeps.
#[must_use]
pub fn build_cut_terrain_mesh(
    terrain: &MapTerrain,
    origin: (f32, f32),
    cell: f32,
    uv_per_tile: bool,
    cut: &TerrainCut,
) -> Mesh {
    let n = terrain.grid_cells as usize;
    let dim = n + 1;
    let span = n as f32 * cell;
    let mut b = Builder {
        origin,
        span,
        cell,
        uv_per_tile,
        positions: Vec::new(),
        uvs: Vec::new(),
        indices: Vec::new(),
    };
    let h = |i: usize, j: usize| terrain.heights.get(j * dim + i).copied().unwrap_or(0.0);
    // The untouched grid keeps its shared vertices (smooth normals as before).
    for j in 0..dim {
        for i in 0..dim {
            b.vertex(i as f32 * cell, j as f32 * cell, h(i, j));
        }
    }
    for j in 0..n {
        for i in 0..n {
            let ids = (
                j * dim + i,
                j * dim + i + 1,
                (j + 1) * dim + i,
                (j + 1) * dim + i + 1,
            );
            let p = |k: usize| {
                let (gi, gj) = (k % dim, k / dim);
                Vec3::new(gi as f32 * cell, gj as f32 * cell, h(gi, gj))
            };
            let candidates = cut.candidates(i as i32, j as i32);
            for tri in [[ids.0, ids.1, ids.2], [ids.1, ids.3, ids.2]] {
                if candidates.is_empty() {
                    b.indices
                        .extend(tri.iter().map(|&k| u32::try_from(k).unwrap_or(0)));
                    continue;
                }
                let ground = [p(tri[0]), p(tri[1]), p(tri[2])];
                let mut outside = vec![ground.map(Vec3::truncate).to_vec()];
                let mut inside: Vec<(Vec<Vec2>, u32)> = Vec::new();
                for &c in candidates {
                    let t = cut.strips.tris[c as usize];
                    let clip = t.map(Vec3::truncate);
                    let mut rest = Vec::new();
                    for poly in outside {
                        let within = clip_to_triangle(&poly, clip);
                        if area(&within) < 1e-5 {
                            rest.push(poly);
                            continue;
                        }
                        inside.push((within, c));
                        // The parts outside the strip triangle, edge by edge.
                        let mut remain = poly;
                        for e in 0..3 {
                            let (r, l) = split(&remain, clip[e], clip[(e + 1) % 3]);
                            if area(&r) >= 1e-5 {
                                rest.push(r);
                            }
                            remain = l;
                        }
                    }
                    outside = rest;
                }
                if inside.is_empty() {
                    b.indices
                        .extend(tri.iter().map(|&k| u32::try_from(k).unwrap_or(0)));
                    continue;
                }
                for poly in &outside {
                    // A sliver left along the seam of two strip triangles lies inside them.
                    let centre = poly.iter().copied().sum::<Vec2>() / poly.len() as f32;
                    if cut.height(centre.x, centre.y).is_some() {
                        continue;
                    }
                    b.polygon(poly, |q| plane_height(&ground, q));
                }
                for (poly, c) in &inside {
                    let t = cut.strips.tris[*c as usize];
                    b.polygon(poly, |q| cut.capped(plane_height(&t, q), q.x, q.y));
                }
            }
        }
    }
    // Steps along the strips' outer edges, split where the grid's triangles change.
    let inside_tile = |p: Vec2| (0.0..=span).contains(&p.x) && (0.0..=span).contains(&p.y);
    for (a3, b3) in &cut.edges {
        let (a, bb) = (a3.truncate(), b3.truncate());
        let mut ts = vec![0.0, 1.0];
        let d = bb - a;
        for k in 0..=n {
            let g = k as f32 * cell;
            for (pa, pd) in [(a.x, d.x), (a.y, d.y)] {
                if pd.abs() > 1e-6 {
                    ts.push((g - pa) / pd);
                }
            }
        }
        for k in 0..=2 * n {
            // Diagonals of the split: x + y = (i + j + 1) cell.
            let g = k as f32 * cell;
            let s = d.x + d.y;
            if s.abs() > 1e-6 {
                ts.push((g - a.x - a.y) / s);
            }
        }
        ts.retain(|t| (0.0..=1.0).contains(t));
        ts.sort_by(f32::total_cmp);
        ts.dedup_by(|x, y| (*x - *y).abs() < 1e-5);
        for w in ts.windows(2) {
            let (p, q) = (a + d * w[0], a + d * w[1]);
            if !inside_tile((p + q) * 0.5) {
                continue;
            }
            let top = (
                cut.capped(a3.z + (b3.z - a3.z) * w[0], p.x, p.y),
                cut.capped(a3.z + (b3.z - a3.z) * w[1], q.x, q.y),
            );
            let ground = (
                terrain_height(terrain, cell, p.x, p.y),
                terrain_height(terrain, cell, q.x, q.y),
            );
            if (ground.0 - top.0).abs().max((ground.1 - top.1).abs()) < STEP_MIN_M {
                continue;
            }
            b.face(p, q, top, ground);
        }
    }
    let Builder {
        positions,
        uvs,
        indices,
        ..
    } = b;
    smooth_normal_mesh(positions, uvs, indices)
}

#[cfg(test)]
mod tests;
