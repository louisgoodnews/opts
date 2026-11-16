//! Tests of the water surface: depth tint, mesh coverage and shore fade.

use super::*;

fn flat_terrain(height: f32) -> MapTerrain {
    MapTerrain {
        grid_cells: 60,
        heights: vec![height; 61 * 61],
    }
}

fn colors_of(mesh: &Mesh) -> Vec<[f32; 4]> {
    match mesh.attribute(Mesh::ATTRIBUTE_COLOR) {
        Some(bevy::mesh::VertexAttributeValues::Float32x4(c)) => c.clone(),
        other => panic!("no rgba colours: {other:?}"),
    }
}

#[test]
fn the_tint_fades_in_from_the_shore_and_darkens_with_depth() {
    let shore = depth_tint(0.0);
    let shallow = depth_tint(1.0);
    let deep = depth_tint(FULL_DEPTH_M);
    let abyss = depth_tint(40.0);
    assert_eq!(
        shore[3], 0.0,
        "a vertex at or above the water line is clear"
    );
    assert_eq!(depth_tint(-3.0)[3], 0.0, "and so is one on dry ground");
    assert!(
        shallow[3] > 0.3 && shallow[3] < deep[3],
        "{shallow:?} {deep:?}"
    );
    assert!((deep[3] - DEEP_ALPHA).abs() < 1e-6);
    assert_eq!(deep, abyss, "no further change below the full depth");
    for k in 0..3 {
        assert!(deep[k] < shallow[k] && shallow[k] < 1.0, "rgb {k} darkens");
    }
    assert_eq!(depth_tint(100.0)[..3], deep[..3]);
}

#[test]
fn every_vertex_carries_its_depth_tint() {
    // 2 m of water over flat ground at -1 m: 3 m deep everywhere.
    let w = MapWater {
        version: 1,
        levels: [2.0; 4],
    };
    let mesh = build_water_mesh(&flat_terrain(-1.0), &w, (0.0, 0.0), 5.0).expect("wet");
    let colors = colors_of(&mesh);
    assert_eq!(colors.len(), mesh.count_vertices());
    let want = depth_tint(3.0);
    assert!(colors.iter().all(|c| *c == want));
    // At a shore the surface fades: the ground rises through the water level.
    let mut t = flat_terrain(0.0);
    for j in 0..61 {
        for i in 0..61 {
            t.heights[j * 61 + i] = i as f32 * 0.2;
        }
    }
    let w = MapWater {
        version: 1,
        levels: [3.0; 4],
    };
    let mesh = build_water_mesh(&t, &w, (0.0, 0.0), 5.0).expect("partly wet");
    let alphas: Vec<f32> = colors_of(&mesh).iter().map(|c| c[3]).collect();
    assert!(alphas.contains(&0.0), "a clear vertex at the shore");
    assert!(alphas.iter().any(|a| *a > 0.4), "and opaque water offshore");
}

#[test]
fn dry_land_gets_no_water() {
    let w = MapWater {
        version: 1,
        levels: [-5.0; 4],
    };
    assert!(build_water_mesh(&flat_terrain(10.0), &w, (0.0, 0.0), 5.0).is_none());
}

#[test]
fn a_flooded_tile_is_covered_completely() {
    let w = MapWater {
        version: 1,
        levels: [3.0; 4],
    };
    let mesh = build_water_mesh(&flat_terrain(0.0), &w, (0.0, 0.0), 5.0).expect("flooded");
    assert_eq!(mesh.count_vertices(), 61 * 61);
    assert_eq!(mesh.indices().expect("indexed").len(), 60 * 60 * 6);
}

#[test]
fn water_stops_where_the_land_rises() {
    // Terrain rises from 0 m in the west to 12 m in the east: a level of 6 m covers the
    // western half only.
    let mut t = flat_terrain(0.0);
    for j in 0..61 {
        for i in 0..61 {
            t.heights[j * 61 + i] = i as f32 * 0.2;
        }
    }
    let w = MapWater {
        version: 1,
        levels: [6.0; 4],
    };
    let mesh = build_water_mesh(&t, &w, (0.0, 0.0), 5.0).expect("partly wet");
    let n = mesh.indices().expect("indexed").len() / 6;
    // Columns 0..30 are wet (the ground at column 30 is exactly 6 m, so column 29's cell
    // reaches it): 30 columns of 60 cells.
    assert_eq!(n, 30 * 60);
}

#[test]
fn a_sloping_surface_follows_the_corner_levels() {
    let w = MapWater {
        version: 1,
        levels: [0.0, 6.0, 0.0, 6.0],
    };
    let mesh = build_water_mesh(&flat_terrain(-1.0), &w, (0.0, 0.0), 5.0).expect("wet");
    let pos = mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .and_then(|a| a.as_float3())
        .expect("positions");
    let max = pos.iter().map(|p| p[1]).fold(f32::MIN, f32::max);
    let min = pos.iter().map(|p| p[1]).fold(f32::MAX, f32::min);
    assert!((max - 6.0).abs() < 1e-4 && min.abs() < 1e-4, "{min} {max}");
}
