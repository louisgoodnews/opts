//! One height service (ISS-071): the game's ground index and the M-0025 ground query answer the
//! same terrain heights, in the same session frame, by construction and by hand-computed values.

use opts_bevy::{GroundElevationQuery, TileTerrain};
use opts_core::formats::map::{MapTerrain, TileGrid};
use opts_ground_height::GroundIndex;

const CELL: f32 = 5.0;

/// A tile whose grid point `(i, j)` lies at the tile-local metres `(i·5, j·5)` with height
/// `f(x, y)`.
fn tile(f: impl Fn(f32, f32) -> f32) -> TileTerrain {
    let mut heights = Vec::with_capacity(61 * 61);
    for j in 0..61 {
        for i in 0..61 {
            heights.push(f(i as f32 * CELL, j as f32 * CELL));
        }
    }
    TileTerrain::new(
        MapTerrain {
            grid_cells: 60,
            heights,
        },
        CELL,
    )
}

/// A plane: 10 m + 0.1 m per metre east + 0.2 m per metre north, in tile-local metres.
fn plane(x: f32, y: f32) -> f32 {
    10.0 + 0.1 * x + 0.2 * y
}

fn grid() -> TileGrid {
    TileGrid::Uniform { pitch: 300.0 }
}

#[test]
fn index_and_query_agree_in_the_start_tile_frame() {
    let start = (1, 0);
    let mut index = GroundIndex::default();
    index.set_frame(start, grid());
    index.insert_tile((1, 0), (300.0, 0.0), Some(tile(plane)), &[]);

    let mut query = GroundElevationQuery::with_start(grid(), start);
    query.insert((1, 0), tile(plane));

    for (x, y) in [(10.0, 20.0), (7.3, 91.1), (150.0, 287.0), (299.9, 0.1)] {
        let from_index = index.terrain_at(x, y).expect("tile (1, 0) is resident");
        let from_query = query.height_at(x, y).expect("tile (1, 0) is resident");
        assert_eq!(from_index, from_query, "({x}, {y})");
        assert!((from_index - plane(x, y)).abs() < 1e-3, "({x}, {y}): {from_index}");
    }
}

#[test]
fn a_hand_computed_height_in_the_start_tile_frame() {
    // Session point (10, 20) of start tile (1, 0) is tile (1, 0) local (10, 20):
    // 10 + 0.1 · 10 + 0.2 · 20 = 15.0.
    let mut index = GroundIndex::default();
    index.set_frame((1, 0), grid());
    index.insert_tile((1, 0), (300.0, 0.0), Some(tile(plane)), &[]);
    let h = index.terrain_at(10.0, 20.0).expect("resident");
    assert!((h - 15.0).abs() < 1e-3, "{h}");
}

#[test]
fn the_default_frame_is_tile_zero_as_before() {
    // Session point (7.3, 91.1) of the default frame: 10 + 0.73 + 18.22 = 28.95.
    let mut index = GroundIndex::default();
    index.insert_tile((0, 0), (0.0, 0.0), Some(tile(plane)), &[]);
    let h = index.terrain_at(7.3, 91.1).expect("resident");
    assert!((h - 28.95).abs() < 1e-3, "{h}");
}

#[test]
fn removing_a_tile_drops_its_height() {
    let mut index = GroundIndex::default();
    index.insert_tile((0, 0), (0.0, 0.0), Some(tile(plane)), &[]);
    index.remove_tile((0, 0));
    assert_eq!(index.terrain_at(7.3, 91.1), None);
}
