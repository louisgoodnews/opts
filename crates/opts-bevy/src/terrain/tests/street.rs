//! The carve of a kerbed street, and the border of two tiles: both sides agree.

use super::*;

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
