//! The terrain mesh: its height follows the triangles, the frame of a shifted tile, and the vertices.

use super::*;

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

#[test]
fn terrain_mesh_has_one_vertex_per_grid_point() {
    let mesh = build_terrain_mesh(&flat(1.0).grid, (300.0, 0.0), 5.0, false);
    assert_eq!(mesh.count_vertices(), 61 * 61);
}
