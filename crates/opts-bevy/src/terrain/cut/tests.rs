//! Tests of the terrain shaped to aligned splines.

use super::super::tests::{flat, spline};
use super::*;
use crate::terrain::TileTerrain;

/// A road 12 m down in a trough: the half profile of `RQ_28_Half.sli` (0 … 12.5 m at +0.1).
fn half_road() -> SplineProfile {
    SplineProfile::parse(b"[heightprofile]\r\n0\r\n12.5\r\n0.1\r\n0.1\r\n").expect("profile")
}

fn trough() -> (TileTerrain, TerrainCut) {
    let ground = flat(0.0);
    // Heading north from (150, 50) for 100 m at -12 m, the strip to the east of the axis.
    let tris = spline_cut_triangles(&half_road(), &spline(150.0, 50.0, -12.0, 100.0), (0.0, 0.0));
    (ground, TerrainCut::new(tris, Vec::new(), 5.0))
}

#[test]
fn the_terrain_lies_on_an_aligned_road_however_deep() {
    let (ground, cut) = trough();
    let shaped = ground.clone().with_cut(cut);
    let inside = shaped.height(156.0, 100.0);
    assert!((inside - (-11.9 - CUT_EPSILON_M)).abs() < 1e-3, "{inside}");
    assert_eq!(
        shaped.height(170.0, 100.0),
        0.0,
        "beside the strip the grid stays"
    );
    assert_eq!(shaped.height(150.0, 20.0), 0.0, "before the road starts");
    assert_eq!(
        ground.height(156.0, 100.0),
        0.0,
        "without the cut nothing changes"
    );
}

#[test]
fn the_mesh_follows_the_strip_and_steps_down_at_its_edges() {
    let (ground, cut) = trough();
    let mesh = build_cut_terrain_mesh(&ground.grid, (0.0, 0.0), 5.0, false, &cut);
    let Some(bevy::mesh::VertexAttributeValues::Float32x3(pos)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        panic!("positions");
    };
    // Only the vertices a triangle uses (the grid keeps its unused points).
    let used: std::collections::HashSet<u32> = mesh
        .indices()
        .expect("indices")
        .iter()
        .map(|i| i as u32)
        .collect();
    let pos: Vec<[f32; 3]> = used.iter().map(|&i| pos[i as usize]).collect();
    // Bevy: (east, height, -north).
    let deep = pos.iter().filter(|p| p[1] < -11.0).count();
    assert!(deep > 0, "vertices at the road's height");
    let covering = pos
        .iter()
        .filter(|p| p[1] > -1.0 && p[0] > 150.5 && p[0] < 162.0 && -p[2] > 51.0 && -p[2] < 149.0)
        .count();
    let bad: Vec<_> = pos
        .iter()
        .filter(|p| p[1] > -1.0 && p[0] > 150.5 && p[0] < 162.0 && -p[2] > 51.0 && -p[2] < 149.0)
        .collect();
    assert_eq!(covering, 0, "no terrain vertex over the trough: {bad:?}");
    // A step along each long edge: faces from -11.93 m up to the ground at 0.
    let step_tops = pos
        .iter()
        .filter(|p| (p[0] - 162.5).abs() < 1e-3 && p[1] > -0.01)
        .count();
    assert!(
        step_tops > 0,
        "a step joins the strip to the ground at its east edge"
    );
}

#[test]
fn two_halves_of_a_road_make_no_step_between_them() {
    let road = half_road();
    let mut tris = spline_cut_triangles(&road, &spline(150.0, 50.0, -12.0, 100.0), (0.0, 0.0));
    let mut west = spline(150.0, 50.0, -12.0, 100.0);
    west.mirrored = true;
    tris.extend(spline_cut_triangles(&road, &west, (0.0, 0.0)));
    let cut = TerrainCut::new(tris, Vec::new(), 5.0);
    assert!(
        cut.edges
            .iter()
            .all(|(a, b)| (a.x - 150.0).abs() > 0.1 || (b.x - 150.0).abs() > 0.1),
        "the axis between the halves is no outer edge"
    );
    assert!(cut.height(150.0, 100.0).is_some());
}

#[test]
fn a_profile_without_height_profile_cuts_nothing() {
    let none = spline_cut_triangles(
        &SplineProfile::default(),
        &spline(150.0, 50.0, -12.0, 100.0),
        (0.0, 0.0),
    );
    assert!(none.is_empty());
    assert!(flat(0.0)
        .with_cut(TerrainCut::new(none, Vec::new(), 5.0))
        .cut
        .is_none());
}

#[test]
fn the_terrain_stays_under_the_drawn_road_on_a_coarser_strip() {
    let ground = vec![[
        Vec3::new(150.0, 40.0, -12.5),
        Vec3::new(170.0, 40.0, -12.5),
        Vec3::new(150.0, 160.0, -12.5),
    ]];
    let tris = spline_cut_triangles(&half_road(), &spline(150.0, 50.0, -12.0, 100.0), (0.0, 0.0));
    let cut = TerrainCut::new(tris, ground, 5.0);
    let h = cut.height(152.0, 60.0).expect("under the strip");
    assert!((h - (-12.5 - CARVE_EPSILON_M)).abs() < 1e-4, "{h}");
}
