//! Mesh construction and polygon clipping for terrain cut to aligned splines.

use super::*;

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

/// One terrain triangle that strips cross: the parts outside the strip triangles keep the ground's
/// plane, the parts inside follow the strips. A triangle no strip really touches is kept whole.
fn cut_triangle(
    b: &mut Builder,
    cut: &TerrainCut,
    (tri, ground): ([usize; 3], [Vec3; 3]),
    candidates: &[u32],
) {
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
        return;
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

/// Steps along the strips' outer edges, split where the grid's triangles change.
fn add_steps(
    b: &mut Builder,
    (terrain, cut): (&MapTerrain, &TerrainCut),
    (cell, n, span): (f32, usize, f32),
) {
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
}

/// The terrain mesh of a tile shaped to `cut` (see the module docs), in absolute Bevy world
/// coordinates like [`super::super::build_terrain_mesh`], whose grid split and UVs it keeps.
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
                cut_triangle(
                    &mut b,
                    cut,
                    (tri, [p(tri[0]), p(tri[1]), p(tri[2])]),
                    candidates,
                );
            }
        }
    }
    add_steps(&mut b, (terrain, cut), (cell, n, span));
    let Builder {
        positions,
        uvs,
        indices,
        ..
    } = b;
    smooth_normal_mesh(positions, uvs, indices)
}
