//! Terrain height grids (`.map.terrain`) → Bevy meshes, the carve under drawn ground surfaces,
//! and the one height function every consumer samples
//! (`docs/src/architecture/terrain-carving.md`).

use bevy::asset::Handle;
use bevy::image::Image;
use bevy::math::{Vec2, Vec3};
use bevy::mesh::Mesh;
use bevy::pbr::StandardMaterial;
use bevy::render::render_resource::Face;

use opts_core::formats::map::{MapSplineInstance, MapTerrain};
use opts_core::formats::scenery::SplineProfile;

use crate::mesh::smooth_normal_mesh;
use crate::scenery::spline_stations;

/// Default side length of a map tile in metres (`docs/src/formats/map/tiles.md` Q-M01). Maps with
/// `[worldcoordinates]` lay their tiles out on Mercator rows instead
/// (`opts_core::formats::map::TileGrid`); a tile's size and cell come from the session's grid.
pub const TILE_SIZE_M: f32 = 300.0;

/// Default terrain grid cell size (60 cells per default tile).
pub const TERRAIN_CELL_M: f32 = TILE_SIZE_M / 60.0;

/// Clearance the carve leaves between a drawn ground surface and the terrain under it (m).
pub const CARVE_EPSILON_M: f32 = 0.03;
/// Deepest a vertex is lowered for one surface (m): terrain further above a surface is buried
/// content (a tunnel, a river bed, stale paving) and stays as authored.
pub const CARVE_COVER_M: f32 = 1.0;

mod align;
pub use align::{align_band, boundary_edges, boundary_edges_out, ALIGN_MAX_M};
pub mod cut;
pub use cut::{build_cut_terrain_mesh, spline_cut_triangles, TerrainCut};

/// A tile's terrain grid with the size of its cells (`TileGrid::cell` of the tile's row): what
/// every height consumer samples.
#[derive(Debug, Clone, PartialEq)]
pub struct TileTerrain {
    /// The height grid.
    pub grid: MapTerrain,
    /// Edge of one cell (m).
    pub cell: f32,
    /// The strips of aligned splines the terrain is shaped to ([`cut`]), when there are any.
    pub cut: Option<std::sync::Arc<TerrainCut>>,
}

impl TileTerrain {
    /// A grid with cells of `cell` metres.
    #[must_use]
    pub fn new(grid: MapTerrain, cell: f32) -> Self {
        Self {
            grid,
            cell,
            cut: None,
        }
    }

    /// The same terrain shaped to the aligned strips of `cut` (none when it is empty).
    #[must_use]
    pub fn with_cut(mut self, cut: TerrainCut) -> Self {
        self.cut = (!cut.is_empty()).then(|| std::sync::Arc::new(cut));
        self
    }

    /// Ground height at tile-local `(x, y)` metres, as drawn: an aligned strip's where one lies,
    /// else the grid's ([`terrain_height`]).
    #[must_use]
    pub fn height(&self, x: f32, y: f32) -> f32 {
        self.cut
            .as_ref()
            .and_then(|c| c.height(x, y))
            .unwrap_or_else(|| terrain_height(&self.grid, self.cell, x, y))
    }
}

/// Height of a tile's terrain at tile-local `(x, y)` metres, interpolated on the triangles the
/// mesh is drawn with ([`build_terrain_mesh`]: cell `(i, j)` splits into `(i,j) (i+1,j) (i,j+1)`
/// and `(i+1,j) (i+1,j+1) (i,j+1)`), so objects, lanes and vehicles stand on the drawn ground.
/// Points outside the grid take the height at the nearest border.
#[must_use]
pub fn terrain_height(terrain: &MapTerrain, cell: f32, x: f32, y: f32) -> f32 {
    let n = terrain.grid_cells as usize;
    let dim = n + 1;
    if n == 0 || terrain.heights.len() < dim * dim {
        return 0.0;
    }
    let gx = (x / cell).clamp(0.0, n as f32);
    let gy = (y / cell).clamp(0.0, n as f32);
    let (i, j) = ((gx as usize).min(n - 1), (gy as usize).min(n - 1));
    let (u, w) = (gx - i as f32, gy - j as f32);
    let h = |i: usize, j: usize| terrain.heights[j * dim + i];
    if u + w <= 1.0 {
        let a = h(i, j);
        a + (h(i + 1, j) - a) * u + (h(i, j + 1) - a) * w
    } else {
        let d = h(i + 1, j + 1);
        d + (h(i, j + 1) - d) * (1.0 - u) + (h(i + 1, j) - d) * (1.0 - w)
    }
}

/// Builds the terrain mesh of one tile in absolute Bevy world coordinates.
///
/// `origin` is the tile's south-west corner in OMSI world metres `(east, north)`, `cell` its
/// terrain cell. With `uv_per_tile` the UVs span 0..1 over the tile (painted ground texture),
/// otherwise one unit per cell (a repeating base texture).
#[must_use]
pub fn build_terrain_mesh(
    terrain: &MapTerrain,
    origin: (f32, f32),
    cell: f32,
    uv_per_tile: bool,
) -> Mesh {
    let dim = terrain.grid_cells as usize + 1;
    let mut positions = Vec::with_capacity(dim * dim);
    let mut uvs = Vec::with_capacity(dim * dim);
    for j in 0..dim {
        for i in 0..dim {
            let h = terrain.heights.get(j * dim + i).copied().unwrap_or(0.0);
            positions.push([origin.0 + i as f32 * cell, h, -(origin.1 + j as f32 * cell)]);
            if uv_per_tile {
                // One texture over the whole tile, row 0 at the north edge.
                let n = (dim - 1) as f32;
                uvs.push([i as f32 / n, 1.0 - j as f32 / n]);
            } else {
                uvs.push([i as f32, j as f32]);
            }
        }
    }
    let mut indices = Vec::with_capacity((dim - 1) * (dim - 1) * 6);
    for j in 0..dim - 1 {
        for i in 0..dim - 1 {
            let a = (j * dim + i) as u32;
            let b = a + 1;
            let c = a + dim as u32;
            let d = c + 1;
            indices.extend_from_slice(&[a, b, c, b, d, c]);
        }
    }
    smooth_normal_mesh(positions, uvs, indices)
}

/// A triangle of a drawn ground surface (a spline's surface ribbon, a crossing's ground face)
/// in a tile's frame: `(east, north, height)` metres.
pub type GroundTriangle = [Vec3; 3];

/// The surface ribbon of a placed spline: its profile's ground surface
/// ([`SplineProfile::ground_surface`]) extruded over the same stations as the drawn spline
/// ([`spline_stations`], `[spline_h]` ramps included). `offset` is the corner of the spline's tile
/// in the frame the triangles are wanted in (zero for the spline's own tile). Mirrored splines
/// negate the profile's `x`; profiles without a surface give nothing.
#[must_use]
pub fn spline_ground_triangles(
    profile: &SplineProfile,
    inst: &MapSplineInstance,
    offset: (f32, f32),
) -> Vec<GroundTriangle> {
    let surface = profile.ground_surface();
    if surface.is_empty() {
        return Vec::new();
    }
    let side = if inst.mirrored { -1.0 } else { 1.0 };
    let stations = spline_stations(inst, offset);
    let at = |k: usize, x: f32, z: f32| {
        let (ex, ny, sz, h, _) = stations[k];
        let x = x * side;
        Vec3::new(ex + h.cos() * x, ny - h.sin() * x, sz + z)
    };
    let mut out = Vec::with_capacity(surface.len() * stations.len() * 2);
    for k in 0..stations.len().saturating_sub(1) {
        for seg in &surface {
            let (a, b) = (at(k, seg.x0, seg.z0), at(k, seg.x1, seg.z1));
            let (c, d) = (at(k + 1, seg.x0, seg.z0), at(k + 1, seg.x1, seg.z1));
            out.push([a, b, c]);
            out.push([b, d, c]);
        }
    }
    out
}

/// What [`carve_terrain`] did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CarveStats {
    /// Vertices lowered.
    pub lowered: usize,
    /// Vertices a surface would have lowered by more than [`CARVE_COVER_M`] (left as authored).
    pub guarded: usize,
    /// Vertices under a surface raised up to just below it (the terrain lay lower than the
    /// surface by at most [`ALIGN_MAX_M`]): the terrain beside the surface edge then starts at
    /// the edge's height instead of a pavement's height lower.
    pub raised: usize,
    /// Vertices changed by the fit of the terrain just outside the outer edges to their height.
    pub fitted: usize,
    /// Vertices outside the surfaces pulled towards the height of the nearest outer edge
    /// ([`align_band`], [`ALIGN_MAX_M`]).
    pub aligned: usize,
}

/// Twice the signed area of the 2D triangle `a b c`.
fn cross2(a: Vec2, b: Vec2, c: Vec2) -> f32 {
    (b - a).perp_dot(c - a)
}

/// The part of convex polygon `subject` inside the counter-clockwise triangle `clip`
/// (Sutherland-Hodgman).
fn clip_to_triangle(subject: &[Vec2], clip: [Vec2; 3]) -> Vec<Vec2> {
    let mut poly = subject.to_vec();
    for e in 0..3 {
        if poly.is_empty() {
            break;
        }
        let (p, q) = (clip[e], clip[(e + 1) % 3]);
        let side = |x: Vec2| cross2(p, q, x);
        let mut next = Vec::with_capacity(poly.len() + 2);
        for k in 0..poly.len() {
            let (a, b) = (poly[k], poly[(k + 1) % poly.len()]);
            let (sa, sb) = (side(a), side(b));
            if sa >= 0.0 {
                next.push(a);
            }
            if (sa >= 0.0) != (sb >= 0.0) {
                next.push(a + (b - a) * (sa / (sa - sb)));
            }
        }
        poly = next;
    }
    poly
}


/// What the carve learns from the surface triangles, per grid vertex: the height it must stay
/// below (`cap`), whether the terrain is far above and left alone (`guarded`) and whether a
/// surface lies over it (`covered`).
struct Carving {
    cap: Vec<f32>,
    guarded: Vec<bool>,
    covered: Vec<bool>,
    n: usize,
    dim: usize,
    cell: f32,
}

impl Carving {
    fn new(n: usize, cell: f32) -> Self {
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
    fn add(&mut self, tri: &GroundTriangle, raw: &[f32]) {
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
                    return;
                }
                let overlap_area: f32 = (1..overlap.len() - 1)
                    .map(|k| cross2(overlap[0], overlap[k], overlap[k + 1]))
                    .sum();
                if overlap_area.abs() < 1e-6 {
                    return;
                }
                let m = overlap
                    .iter()
                    .map(|&x| height_at(x))
                    .fold(f32::INFINITY, f32::min)
                    - CARVE_EPSILON_M;
                for (a, b) in corners {
                    if !(0..=n as i64).contains(&a) || !(0..=n as i64).contains(&b) {
                        return;
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

/// Lowers the terrain under drawn ground surfaces so it never covers them
/// (`docs/src/architecture/terrain-carving.md` §4).
///
/// For every surface triangle and every terrain triangle it overlaps (the mesh's own split), the
/// lowest surface height on their overlap minus [`CARVE_EPSILON_M`] caps the terrain triangle's
/// three vertices. A cap that would lower a vertex by more than [`CARVE_COVER_M`] is not applied
/// (buried content). Terrain triangles one cell beyond the tile's border take part too, so a
/// border vertex gets the same caps on both tiles that share it. Heights are only ever lowered;
/// the mesh topology is unchanged.
#[must_use]
pub fn carve_terrain(
    terrain: &TileTerrain,
    surfaces: &[GroundTriangle],
) -> (TileTerrain, CarveStats) {
    let n = terrain.grid.grid_cells as usize;
    let dim = n + 1;
    let mut out = terrain.clone();
    let mut stats = CarveStats::default();
    if n == 0 || terrain.grid.heights.len() < dim * dim {
        return (out, stats);
    }
    let cell = terrain.cell;
    let raw = &terrain.grid.heights;
    let mut cv = Carving::new(n, cell);
    for tri in surfaces {
        cv.add(tri, raw);
    }
    for (k, h) in out.grid.heights.iter_mut().enumerate().take(dim * dim) {
        if cv.cap[k] < *h {
            *h = cv.cap[k];
            stats.lowered += 1;
        } else if cv.guarded[k] {
            stats.guarded += 1;
        }
    }
    // A vertex under a surface that lies lower than the surface is raised to just below it
    // (never above its cap, so the terrain still covers nothing): the terrain triangles that
    // reach out from under the surface then start at the surface's height at its edge, not 10-25
    // cm lower. A bigger gap is content (a bridge, a cutting) and stays.
    for (k, h) in out.grid.heights.iter_mut().enumerate().take(dim * dim) {
        if cv.covered[k]
            && !cv.guarded[k]
            && cv.cap[k].is_finite()
            && *h < cv.cap[k]
            && cv.cap[k] - *h <= ALIGN_MAX_M
        {
            *h = cv.cap[k];
            stats.raised += 1;
        }
    }
    // The terrain beside the surfaces follows their outer edges.
    let outer = align::boundary_edges_out(surfaces);
    stats.aligned = align::align_vertices(
        outer.iter().map(|(a, b, _)| (*a, *b)).collect(),
        &mut out.grid.heights,
        &cv.cap,
        &cv.covered,
        &cv.guarded,
        n,
        cell,
    );
    // … and the terrain just outside every edge is at the edge's height.
    stats.fitted = align::fit_edges(&outer, &mut out.grid.heights, raw, &cv.cap, &cv.guarded, n, cell);
    (out, stats)
}

#[must_use]
pub fn terrain_material(texture: Option<Handle<Image>>) -> StandardMaterial {
    StandardMaterial {
        base_color_texture: texture,
        perceptual_roughness: 0.95,
        metallic: 0.0,
        cull_mode: Some(Face::Back),
        ..Default::default()
    }
}

#[cfg(test)]
pub(super) mod tests;
