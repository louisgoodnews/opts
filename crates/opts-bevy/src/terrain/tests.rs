//! Unit tests of the terrain carve, mesh and height function.

use super::*;
use opts_core::formats::scenery::{SplineGraphicalProfile, SplineProfilePoint};

pub(super) fn flat(h: f32) -> TileTerrain {
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
pub(super) fn spline(x: f32, y: f32, z: f32, length: f32) -> MapSplineInstance {
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
