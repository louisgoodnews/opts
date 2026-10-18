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

/// A tile's terrain grid with the size of its cells (`TileGrid::cell` of the tile's row): what
/// every height consumer samples.
#[derive(Debug, Clone, PartialEq)]
pub struct TileTerrain {
    /// The height grid.
    pub grid: MapTerrain,
    /// Edge of one cell (m).
    pub cell: f32,
}

impl TileTerrain {
    /// A grid with cells of `cell` metres.
    #[must_use]
    pub fn new(grid: MapTerrain, cell: f32) -> Self {
        Self { grid, cell }
    }

    /// Ground height at tile-local `(x, y)` metres, as drawn ([`terrain_height`]).
    #[must_use]
    pub fn height(&self, x: f32, y: f32) -> f32 {
        terrain_height(&self.grid, self.cell, x, y)
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
    let mut cap = vec![f32::INFINITY; dim * dim];
    let mut guarded = vec![false; dim * dim];
    let ring = -1_i64..=n as i64;
    for tri in surfaces {
        let p = tri.map(|v| v.truncate());
        let area = cross2(p[0], p[1], p[2]);
        if area.abs() < 1e-8 {
            continue;
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
                            guarded[k] = true;
                        } else if m < cap[k] {
                            cap[k] = m;
                        }
                    }
                }
            }
        }
    }
    for (k, h) in out.grid.heights.iter_mut().enumerate().take(dim * dim) {
        if cap[k] < *h {
            *h = cap[k];
            stats.lowered += 1;
        } else if guarded[k] {
            stats.guarded += 1;
        }
    }
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
mod tests {
    use super::*;
    use opts_core::formats::scenery::{SplineGraphicalProfile, SplineProfilePoint};

    fn flat(h: f32) -> TileTerrain {
        TileTerrain::new(
            MapTerrain {
                grid_cells: 60,
                heights: vec![h; 61 * 61],
            },
            5.0,
        )
    }

    fn profile(parts: &[&[(f32, f32)]]) -> SplineProfile {
        let mut p = SplineProfile::default();
        for pts in parts {
            p.graphical_profiles.push(SplineGraphicalProfile {
                texture_index: 0,
                points: pts
                    .iter()
                    .map(|&(x, z)| SplineProfilePoint {
                        x,
                        z,
                        u: 0.0,
                        v_slope: 0.0,
                    })
                    .collect(),
            });
        }
        p
    }

    /// A straight spline heading north from `(x, y)` at height `z`.
    fn spline(x: f32, y: f32, z: f32, length: f32) -> MapSplineInstance {
        MapSplineInstance {
            flag: 0,
            sli_file: String::new(),
            id_code: 1,
            conn_id_1: 0,
            conn_id_2: 0,
            position: Vec3::new(x, y, z),
            heading_deg: 0.0,
            length_m: length,
            radius_m: 0.0,
            gradient_start: 0.0,
            gradient_end: 0.0,
            has_height_profile: false,
            mirrored: false,
            raw_params: Vec::new(),
        }
    }

    fn h(t: &TileTerrain, i: usize, j: usize) -> f32 {
        t.grid.heights[j * 61 + i]
    }

    #[test]
    fn terrain_height_follows_the_mesh_triangles() {
        let mut heights = vec![0.0; 61 * 61];
        for (k, v) in heights.iter_mut().enumerate() {
            *v = ((k * 7919) % 13) as f32;
        }
        let t = TileTerrain::new(
            MapTerrain {
                grid_cells: 60,
                heights,
            },
            5.0,
        );
        let mesh = build_terrain_mesh(&t.grid, (0.0, 0.0), 5.0, false);
        let pos = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(|a| a.as_float3())
            .expect("positions")
            .to_vec();
        let Some(bevy::mesh::Indices::U32(idx)) = mesh.indices() else {
            panic!("indices")
        };
        // Both triangles of cell (3, 4): sample their centroids and interpolate the mesh's own
        // triangle there.
        let cell = 4 * 60 + 3;
        for tri in [
            &idx[cell * 6..cell * 6 + 3],
            &idx[cell * 6 + 3..cell * 6 + 6],
        ] {
            let v: Vec<Vec3> = tri.iter().map(|&i| Vec3::from(pos[i as usize])).collect();
            for wts in [[1.0 / 3.0; 3], [0.6, 0.3, 0.1], [0.1, 0.1, 0.8]] {
                let p = v[0] * wts[0] + v[1] * wts[1] + v[2] * wts[2];
                let got = t.height(p.x, -p.z);
                assert!((got - p.y).abs() < 1e-4, "{got} vs mesh {}", p.y);
            }
        }
    }

    /// `Damm1`-like embankment: crest ±5 m at the datum, flanks down to −20 m at ±35 m, crest 15 m
    /// above flat ground. Nothing beyond crest ± 5 m changes and nothing is lowered by more than
    /// the guard (today's carve dug a 20 m pit 38 m wide).
    #[test]
    fn an_embankment_carves_only_its_crest() {
        let ground = flat(10.0);
        let damm = profile(&[
            &[(-5.0, 0.0), (5.0, 0.0)],
            &[(-35.0, -20.0), (-5.0, 0.0)],
            &[(5.0, 0.0), (35.0, -20.0)],
        ]);
        // The crest at 9.5 m: terrain 0.5 m above it is lowered under the crest only.
        let tris = spline_ground_triangles(&damm, &spline(150.0, 50.0, 9.5, 200.0), (0.0, 0.0));
        let (carved, stats) = carve_terrain(&ground, &tris);
        assert!(stats.lowered > 0);
        for j in 0..=60 {
            for i in 0..=60 {
                let (x, d) = (i as f32 * 5.0, h(&ground, i, j) - h(&carved, i, j));
                assert!(d <= CARVE_COVER_M + 1e-5);
                if (x - 150.0).abs() > 10.0 {
                    assert_eq!(d, 0.0, "vertex {i},{j} at x {x} lowered by {d}");
                }
            }
        }
        // The crest 15 m above the ground: nothing to carve at all.
        let tris = spline_ground_triangles(&damm, &spline(150.0, 50.0, 25.0, 200.0), (0.0, 0.0));
        assert_eq!(carve_terrain(&ground, &tris).0, ground);
    }

    /// A carriageway (0.0) between kerbs and pavements (+0.15): the terrain baked to the datum
    /// ends below the surface everywhere, a vertex 0.5 m above the outer edge is lowered just
    /// enough, a vertex two cells out stays.
    #[test]
    fn a_kerbed_street_is_carved_just_enough() {
        let street = profile(&[&[
            (-6.0, 0.15),
            (-4.0, 0.15),
            (-4.0, 0.0),
            (4.0, 0.0),
            (4.0, 0.15),
            (6.0, 0.15),
        ]]);
        let mut ground = flat(10.0);
        // An outer vertex (x = 160) 0.5 m above the pavement's outer edge (x = 156, 10.15 m).
        ground.grid.heights[20 * 61 + 32] = 10.65;
        let inst = spline(150.0, 50.0, 10.0, 200.0);
        let tris = spline_ground_triangles(&street, &inst, (0.0, 0.0));
        let (carved, _) = carve_terrain(&ground, &tris);
        for y in [60.0_f32, 100.0, 133.3] {
            for x in [146.5_f32, 150.0, 153.9, 155.0] {
                let surface = 10.0 + street.surface_at(x - 150.0).expect("on the street");
                assert!(
                    carved.height(x, y) <= surface - CARVE_EPSILON_M + 1e-4,
                    "terrain {} over surface {surface} at ({x}, {y})",
                    carved.height(x, y)
                );
            }
        }
        // Lowered just below the pavement it shares triangles with, no further.
        assert!((h(&carved, 32, 20) - (10.15 - CARVE_EPSILON_M)).abs() < 1e-4);
        // Two cells beyond the pavement edge (x = 165, 170): untouched.
        assert_eq!(h(&carved, 33, 20), 10.0);
        assert_eq!(h(&carved, 34, 20), 10.0);
    }

    /// The streamer keeps each tile's triangles in the tile's own frame and moves them by the
    /// neighbour's offset (M-0089 step 2): that equals asking for the offset directly.
    #[test]
    fn shifting_the_own_frame_triangles_equals_asking_for_the_offset() {
        let road = profile(&[&[(-4.0, 0.0), (4.0, 0.0), (4.0, 0.15), (6.0, 0.15)]]);
        let mut inst = spline(150.0, 50.0, 10.0, 200.0);
        inst.radius_m = 120.0;
        inst.gradient_end = 0.02;
        let (ox, oy) = (-371.9_f32, 371.9_f32);
        let direct = spline_ground_triangles(&road, &inst, (ox, oy));
        let shift = Vec3::new(ox, oy, 0.0);
        let shifted: Vec<GroundTriangle> = spline_ground_triangles(&road, &inst, (0.0, 0.0))
            .iter()
            .map(|t| t.map(|p| p + shift))
            .collect();
        assert!(!direct.is_empty());
        assert_eq!(direct.len(), shifted.len());
        for (d, s) in direct.iter().zip(&shifted) {
            for (p, q) in d.iter().zip(s) {
                assert!((*p - *q).length() < 1e-3, "{p} against {q}");
            }
        }
    }

    /// Three metres of cover over a surface (a tunnel roof): the guard keeps it.
    #[test]
    fn deep_cover_is_left_alone() {
        let ground = flat(13.0);
        let road = profile(&[&[(-4.0, 0.0), (4.0, 0.0)]]);
        let tris = spline_ground_triangles(&road, &spline(150.0, 50.0, 10.0, 100.0), (0.0, 0.0));
        let (carved, stats) = carve_terrain(&ground, &tris);
        assert_eq!(carved, ground);
        assert!(stats.guarded > 0);
        assert_eq!(stats.lowered, 0);
    }

    /// An asymmetric profile (surface on the right only) lands on the left when mirrored.
    #[test]
    fn a_mirrored_profile_carves_the_other_side() {
        let ground = flat(10.2);
        let half = profile(&[&[(2.0, 0.0), (12.0, 0.0)]]);
        let mut inst = spline(150.0, 50.0, 10.0, 100.0);
        let (right, _) = carve_terrain(&ground, &spline_ground_triangles(&half, &inst, (0.0, 0.0)));
        inst.mirrored = true;
        let (left, _) = carve_terrain(&ground, &spline_ground_triangles(&half, &inst, (0.0, 0.0)));
        assert!(right.height(157.0, 100.0) < 10.0);
        assert_eq!(right.height(143.0, 100.0), 10.2);
        assert!(left.height(143.0, 100.0) < 10.0);
        assert_eq!(left.height(157.0, 100.0), 10.2);
    }

    /// A `[spline_h]` ramp rises 2 m as a cubic; the carve follows the ramp, not a straight line.
    #[test]
    fn a_spline_h_ramp_is_followed() {
        let ground = flat(11.2);
        let road = profile(&[&[(-4.0, 0.0), (4.0, 0.0)]]);
        let mut inst = spline(150.0, 50.0, 10.0, 100.0);
        inst.has_height_profile = true;
        inst.raw_params = vec![String::new(), String::new(), "2".into()];
        let tris = spline_ground_triangles(&road, &inst, (0.0, 0.0));
        let (carved, _) = carve_terrain(&ground, &tris);
        // Hermite between flat ends: 25 % along, the road is at 10 + 2·(3·0.0625 − 2·0.015625)
        // = 10.3125 (a straight ramp would be at 10.5).
        let at = carved.height(150.0, 75.0);
        assert!(at <= 10.3125 - CARVE_EPSILON_M + 0.05, "{at}");
        assert!(at > 10.0, "{at}");
        // Near the top the terrain (11.2) is within 0.03 + rounding of the road surface.
        assert!(carved.height(150.0, 145.0) > 11.0);
    }

    /// A road on the western neighbour that runs on into this tile is carved here too, and the
    /// vertices of the shared border get the same height on both tiles.
    #[test]
    fn border_vertices_agree_between_tiles() {
        let ground = flat(10.5);
        let road = profile(&[&[(-3.0, 0.0), (3.0, 0.0)]]);
        // Diagonal road on the west tile crossing its east border at y ≈ 150.
        let mut inst = spline(260.0, 110.0, 10.0, 120.0);
        inst.heading_deg = 45.0;
        let west = carve_terrain(&ground, &spline_ground_triangles(&road, &inst, (0.0, 0.0))).0;
        let east = carve_terrain(
            &ground,
            &spline_ground_triangles(&road, &inst, (-300.0, 0.0)),
        )
        .0;
        let mut lowered = 0;
        for j in 0..=60 {
            let (w, e) = (h(&west, 60, j), h(&east, 0, j));
            assert!((w - e).abs() < 1e-4, "row {j}: west {w} east {e}");
            lowered += usize::from(w < 10.5);
        }
        assert!(lowered >= 2, "the road crosses the border");
        assert!(east.height(10.0, 160.0) < 10.0);
    }

    #[test]
    fn terrain_mesh_has_one_vertex_per_grid_point() {
        let mesh = build_terrain_mesh(&flat(1.0).grid, (300.0, 0.0), 5.0, false);
        assert_eq!(mesh.count_vertices(), 61 * 61);
    }
}
