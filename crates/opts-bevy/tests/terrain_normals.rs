//! Terrain normals from the height field and their stitching across a tile border (M-0025 Phase
//! B; `docs/src/architecture/terrain-rendering.md` section 2). Expected values by hand: a cell is
//! 5 m, so a grid point's slope is (height difference) / (10 m) across the point or / (5 m) at an
//! edge with one side only.

use bevy::math::Vec3;
use opts_bevy::terrain::{field_normals, TileBeyond};
use opts_core::formats::map::MapTerrain;

const CELL: f32 = 5.0;
const DIM: usize = 61;

/// A 60-cell grid whose grid point `(i, j)` lies at the metres `(i·5, j·5)` of `f`.
fn grid(f: impl Fn(f32, f32) -> f32) -> MapTerrain {
    let mut heights = Vec::with_capacity(DIM * DIM);
    for j in 0..DIM {
        for i in 0..DIM {
            heights.push(f(i as f32 * CELL, j as f32 * CELL));
        }
    }
    MapTerrain {
        grid_cells: 60,
        heights,
    }
}

fn at(normals: &[[f32; 3]], i: usize, j: usize) -> Vec3 {
    Vec3::from(normals[j * DIM + i])
}

fn assert_vec(got: Vec3, want: Vec3) {
    assert!((got - want).length() < 1e-4, "{got:?} against {want:?}");
}

#[test]
fn a_flat_tile_has_straight_up_normals_everywhere() {
    let flat = grid(|_, _| 7.0);
    let normals = field_normals(&flat, CELL, &TileBeyond::default());
    assert_eq!(normals.len(), DIM * DIM);
    for n in &normals {
        assert_vec(Vec3::from(*n), Vec3::Y);
    }
}

#[test]
fn a_slope_to_the_north_tilts_the_normal_to_the_south() {
    // Height rises 1 m per row of 5 m: slope 0.2 towards the north (Bevy +z is south).
    let rising = grid(|_, y| 0.2 * y);
    let normals = field_normals(&rising, CELL, &TileBeyond::default());
    let want = Vec3::new(0.0, 1.0, 0.2).normalize();
    assert_vec(at(&normals, 30, 30), want);
    // The south edge has no row below: its one-sided slope is the same 0.2.
    assert_vec(at(&normals, 30, 0), want);
}

#[test]
fn a_slope_to_the_east_tilts_the_normal_to_the_west() {
    // Height 0.1 m per metre east: slope 0.1, so the normal leans to -x.
    let rising = grid(|x, _| 0.1 * x);
    let normals = field_normals(&rising, CELL, &TileBeyond::default());
    assert_vec(at(&normals, 30, 30), Vec3::new(-0.1, 1.0, 0.0).normalize());
}

#[test]
fn the_east_border_takes_the_neighbour_beyond_it() {
    // A quadratic ground 0.01·x² in absolute metres; this tile covers x in 0..300 and the east
    // neighbour x in 300..600. At the border x = 300 the central difference over 295..305 is
    // (930.25 - 870.25) / 10 = 6.0; the one-sided difference inside the tile is
    // (900 - 870.25) / 5 = 5.95.
    let this = grid(|x, _| 0.01 * x * x);
    let east = grid(|x, _| 0.01 * (300.0 + x) * (300.0 + x));
    let beyond = TileBeyond::from_neighbours(60, None, Some(&east), None, None);

    let stitched = field_normals(&this, CELL, &beyond);
    assert_vec(at(&stitched, 60, 30), Vec3::new(-6.0, 1.0, 0.0).normalize());

    let alone = field_normals(&this, CELL, &TileBeyond::default());
    assert_vec(at(&alone, 60, 30), Vec3::new(-5.95, 1.0, 0.0).normalize());
}

#[test]
fn beyond_takes_the_neighbour_column_or_row_next_to_the_border() {
    // Marker values: a grid point's value is its column index (or row index) in its own tile.
    let marked = |f: fn(usize, usize) -> f32| MapTerrain {
        grid_cells: 60,
        heights: (0..DIM * DIM).map(|k| f(k % DIM, k / DIM)).collect(),
    };
    let west = marked(|i, _| i as f32);
    let east = marked(|i, _| i as f32);
    let south = marked(|_, j| j as f32);
    let north = marked(|_, j| j as f32);
    let b = TileBeyond::from_neighbours(60, Some(&west), Some(&east), Some(&south), Some(&north));
    assert_eq!(b.west.as_ref().map(|c| c[7]), Some(59.0));
    assert_eq!(b.east.as_ref().map(|c| c[7]), Some(1.0));
    assert_eq!(b.south.as_ref().map(|r| r[7]), Some(59.0));
    assert_eq!(b.north.as_ref().map(|r| r[7]), Some(1.0));
}

#[test]
fn a_neighbour_of_another_size_or_missing_is_not_held() {
    let small = MapTerrain {
        grid_cells: 30,
        heights: vec![0.0; 31 * 31],
    };
    let b = TileBeyond::from_neighbours(60, Some(&small), None, None, None);
    assert_eq!(b, TileBeyond::default());
}
