//! Ground elevation queries on synthetic terrain: exact interpolation, continuity across tile
//! borders, normals (M-0025, `docs/src/architecture/terrain-rendering.md` section 5).

use opts_bevy::{terrain_cell, tile_size, GroundElevationQuery};
use opts_core::formats::map::MapTerrain;

/// A tile whose height at grid point `(i, j)` is `f(x, y)` for the metres `x, y` of the point
/// within the tile (default tile size 300 m, 5 m cells).
fn tile(origin: (f32, f32), f: impl Fn(f32, f32) -> f32) -> MapTerrain {
    let cell = terrain_cell();
    let mut heights = Vec::with_capacity(61 * 61);
    for j in 0..61 {
        for i in 0..61 {
            heights.push(f(origin.0 + i as f32 * cell, origin.1 + j as f32 * cell));
        }
    }
    MapTerrain {
        grid_cells: 60,
        heights,
    }
}

#[test]
fn bilinear_interpolation_reproduces_a_plane_exactly() {
    assert_eq!(tile_size(), 300.0, "these tests assume the default tile size");
    let plane = |x: f32, y: f32| 12.0 + 0.1 * x + 0.2 * y;
    let mut q = GroundElevationQuery::new();
    q.insert((0, 0), tile((0.0, 0.0), plane));
    for (x, y) in [(0.0, 0.0), (7.3, 91.1), (150.0, 150.0), (299.9, 0.1), (123.456, 287.0)] {
        let h = q.height_at(x, y).unwrap();
        assert!((h - plane(x, y)).abs() < 1e-3, "({x}, {y}): {h} against {}", plane(x, y));
    }
}

#[test]
fn heights_are_continuous_across_a_shared_tile_border() {
    let ground = |x: f32, y: f32| 30.0 + (x * 0.05).sin() * 3.0 + 0.02 * y;
    let mut q = GroundElevationQuery::new();
    q.insert((0, 0), tile((0.0, 0.0), ground));
    q.insert((1, 0), tile((300.0, 0.0), ground));
    for y in [3.0, 77.7, 150.0, 299.0] {
        let (west, east) = (q.height_at(299.999, y).unwrap(), q.height_at(300.001, y).unwrap());
        assert!((west - east).abs() < 0.01, "step of {} m at y = {y}", (west - east).abs());
    }
}

#[test]
fn points_outside_the_held_tiles_have_no_height_and_removal_forgets_a_tile() {
    let mut q = GroundElevationQuery::new();
    q.insert((0, 0), tile((0.0, 0.0), |_, _| 5.0));
    assert!(q.height_at(-1.0, 10.0).is_none());
    assert!(q.height_at(10.0, 301.0).is_none());
    assert_eq!(q.height_at(10.0, 10.0), Some(5.0));
    q.remove((0, 0));
    assert!(q.is_empty() && q.height_at(10.0, 10.0).is_none());
}

#[test]
fn normals_follow_the_slope_also_across_a_border() {
    let plane = |x: f32, y: f32| 0.1 * x - 0.05 * y;
    let mut q = GroundElevationQuery::new();
    q.insert((0, 0), tile((0.0, 0.0), plane));
    q.insert((1, 0), tile((300.0, 0.0), plane));
    let expect = bevy::math::Vec3::new(-0.1, 0.05, 1.0).normalize();
    for (x, y) in [(100.0, 100.0), (299.5, 100.0), (300.5, 40.0)] {
        let n = q.normal_at(x, y).unwrap();
        assert!((n - expect).length() < 1e-3, "({x}, {y}): {n:?} against {expect:?}");
    }
    let flat = {
        let mut f = GroundElevationQuery::new();
        f.insert((0, 0), tile((0.0, 0.0), |_, _| 3.0));
        f
    };
    assert!((flat.normal_at(150.0, 150.0).unwrap() - bevy::math::Vec3::Z).length() < 1e-6);
}
