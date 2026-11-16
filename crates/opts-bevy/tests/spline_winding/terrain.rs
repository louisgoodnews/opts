//! Terrain under a road is carved, and only ground surfaces count.

use super::*;

fn carve(heights: f32, prof: &SplineProfile, inst: &MapSplineInstance) -> MapTerrain {
    let terrain = TileTerrain::new(
        MapTerrain {
            grid_cells: 60,
            heights: vec![heights; 61 * 61],
        },
        TERRAIN_CELL_M,
    );
    carve_terrain(&terrain, &spline_ground_triangles(prof, inst, (0.0, 0.0)))
        .0
        .grid
}

#[test]
fn terrain_never_covers_a_road() {
    // Flat terrain 0.25 m above the road datum (the Waldhofen curb-strip situation).
    let dim = 61;
    let inst = instance(0.0, 0.0, 0.0, false);
    let prof = profile(vec![pt(-4.0, 0.1), pt(4.0, 0.1)]);
    let carved = carve(10.25, &prof, &inst);
    // Road runs north from (50, 50) for 40 m, 8 m wide, surface at 10.1.
    for j in 0..dim {
        for i in 0..dim {
            let (x, y) = (i as f32 * TERRAIN_CELL_M, j as f32 * TERRAIN_CELL_M);
            let h = carved.heights[j * dim + i];
            if (46.0..=54.0).contains(&x) && (50.0..=90.0).contains(&y) {
                assert!(h < 10.1, "terrain {h} covers the road at ({x}, {y})");
            }
            if !(40.0..=60.0).contains(&x) {
                assert_eq!(h, 10.25, "terrain away from the road must stay as authored");
            }
        }
    }
}

#[test]
fn splines_without_ground_surface_leave_terrain_alone() {
    // A wall: vertical faces and a top 2 m up are no ground surface.
    let prof = profile(vec![
        pt(-0.2, 0.0),
        pt(-0.2, 2.0),
        pt(0.2, 2.0),
        pt(0.2, 0.0),
    ]);
    let carved = carve(10.5, &prof, &instance(0.0, 0.0, 0.0, false));
    assert!(carved.heights.iter().all(|h| *h == 10.5));
}

#[test]
fn flat_ground_surfaces_are_not_covered_but_cuttings_stay() {
    // An 8 m pavement without height profile, terrain 8 cm above it: lowered onto it.
    let mut prof = profile(vec![pt(-4.0, 0.0), pt(4.0, 0.0)]);
    prof.height_profiles.clear();
    let carved = carve(10.08, &prof, &instance(0.0, 0.0, 0.0, false));
    assert!(carved.heights.iter().any(|h| *h < 10.0));
    // Terrain 2 m above it (a hill over a tunnel) stays as authored.
    let carved = carve(12.0, &prof, &instance(0.0, 0.0, 0.0, false));
    assert!(carved.heights.iter().all(|h| *h == 12.0));
}
