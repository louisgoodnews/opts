//! Unit tests for `super` (`align.rs`).

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
    let under = at(&carved, 9, 20);
    assert!(
        (under - 0.27).abs() < 1e-4,
        "x = 45 is under the surface: raised to just below it ({under})"
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

#[test]
fn the_drawn_terrain_just_outside_an_edge_is_at_the_edges_height() {
    // The surface at 0.3 m, the terrain 0.25 m lower and flat: after the carve, the
    // alignment and the fit the terrain 0.4 m outside the edge at x = 41 is within a
    // centimetre of the edge's height (less the carve's clearance).
    let (carved, stats) = carve_terrain(&flat(0.05), &strip(41.0, 49.0, 0.3));
    assert!(stats.fitted > 0);
    for y in [60.0_f32, 120.0, 200.0] {
        for x in [40.6_f32, 49.4] {
            let h = carved.height(x, y);
            assert!((h - 0.27).abs() < 0.015, "({x}, {y}): {h}");
        }
    }
}

#[test]
fn a_gap_that_is_content_is_left_alone() {
    // A surface 3 m above the terrain (a bridge deck): nothing outside it moves.
    let (carved, _) = carve_terrain(&flat(0.0), &strip(41.0, 49.0, 3.0));
    assert_eq!(carved.height(40.6, 100.0), 0.0);
}
