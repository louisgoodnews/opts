//! Mesh construction and polygon clipping for terrain cut to aligned splines.

use super::*;
mod clip;
mod steps;
mod triangles;

use clip::area;
use steps::add_steps;
use triangles::cut_triangle;

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
/// coordinates like [`super::super::build_terrain_mesh`], whose grid split and UVs it keeps.
#[must_use]
pub fn build_cut_terrain_mesh(
    terrain: &MapTerrain,
    origin: (f32, f32),
    cell: f32,
    uv_per_tile: bool,
    cut: &TerrainCut,
) -> Mesh {
    build_cut_terrain_mesh_with(
        terrain,
        origin,
        cell,
        uv_per_tile,
        cut,
        &TileBeyond::default(),
    )
}

/// [`build_cut_terrain_mesh`] with the grid's normals stitched to the held neighbours (see
/// [`apply_field_normals`]).
#[must_use]
pub fn build_cut_terrain_mesh_with(
    terrain: &MapTerrain,
    origin: (f32, f32),
    cell: f32,
    uv_per_tile: bool,
    cut: &TerrainCut,
    beyond: &TileBeyond,
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
    let mut mesh = smooth_normal_mesh(positions, uvs, indices);
    apply_field_normals(&mut mesh, terrain, cell, beyond);
    mesh
}
