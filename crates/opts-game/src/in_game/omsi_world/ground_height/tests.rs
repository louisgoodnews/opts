//! Unit tests for `super` (`ground_height.rs`).

use super::*;
use opts_core::formats::scenery::SurfaceSegment;
use std::sync::Arc;

fn sample(mirrored: bool) -> RoadSample {
    RoadSample {
        x: 100.0,
        y: 200.0,
        z: 10.0,
        half_width: 4.0,
        heading: 0.0, // north: right is east
        surface: Arc::new(vec![
            SurfaceSegment {
                x0: -3.0,
                z0: 0.12,
                x1: 0.0,
                z1: 0.12,
            },
            SurfaceSegment {
                x0: 0.0,
                z0: 0.12,
                x1: 3.0,
                z1: 0.0,
            },
        ]),
        mirrored,
        along_m: 0.0,
        strips: Arc::default(),
    }
}

/// M-0097/M-0090: a vehicle stands on the drawn road surface (the datum plus the profile's
/// surface at its lateral offset), not on the datum: a cobbled road 12 cm thick used to
/// sink the wheels by that much.
#[test]
fn the_height_is_the_datum_plus_the_profiles_surface_at_the_lateral_offset() {
    let s = sample(false);
    assert!(
        (s.surface_z(98.0, 200.0) - 10.12).abs() < 1e-5,
        "left of the centre line"
    );
    assert!(
        (s.surface_z(101.5, 200.0) - 10.06).abs() < 1e-5,
        "on the sloping right half"
    );
    assert!(
        (s.surface_z(100.0, 230.0) - 10.12).abs() < 1e-5,
        "along the road: same lateral"
    );
    assert_eq!(
        s.surface_z(110.0, 200.0),
        10.0,
        "off the profile: the datum"
    );
}

#[test]
fn a_mirrored_spline_reads_its_profile_the_other_way_round() {
    let s = sample(true);
    // Mirrored: the profile's right half (sloping down to the datum) lies on the left.
    assert!((s.surface_z(98.5, 200.0) - 10.06).abs() < 1e-5);
    assert!((s.surface_z(102.0, 200.0) - 10.12).abs() < 1e-5);
}

#[test]
fn the_heading_turns_the_lateral_axis() {
    let mut s = sample(false);
    s.heading = std::f32::consts::FRAC_PI_2; // east: right is south
    assert!(
        (s.surface_z(100.0, 202.0) - 10.12).abs() < 1e-5,
        "north of the line is left"
    );
    assert!(
        (s.surface_z(100.0, 198.5) - 10.06).abs() < 1e-5,
        "south is the sloping right half"
    );
}

/// The deck of a bridge over bare ground is not a floor for a vehicle that stands on the
/// ground, and still is one for a vehicle that stands on it.
#[test]
fn a_deck_over_the_apron_is_not_climbed_but_is_driven_on() {
    use opts_bevy::TileTerrain;
    use opts_core::formats::map::MapTerrain;
    let terrain = TileTerrain::new(
        MapTerrain {
            grid_cells: 60,
            heights: vec![0.07; 61 * 61],
        },
        5.0,
    );
    let mut index = GroundIndex::default();
    index.terrain.insert((0, 0), terrain);
    let mut deck = sample(false);
    deck.z = 6.0;
    deck.x = 100.0;
    deck.y = 100.0;
    deck.half_width = 20.0;
    deck.surface = Arc::new(Vec::new());
    index
        .cells
        .entry(cell_of(100.0, 100.0))
        .or_default()
        .push(((0, 0), deck));
    let on_ground = index.height_at(100.0, 100.0, 0.07).expect("ground");
    assert!((on_ground - 0.07).abs() < 0.01, "{on_ground}");
    let on_deck = index.height_at(100.0, 100.0, 6.0).expect("deck");
    assert!((on_deck - 6.0).abs() < 0.01, "{on_deck}");
}

/// A street plate a few decimetres over the terrain is the ground (the bus drives on it);
/// a deck 6 m up is not (the bus stays on the terrain under it).
#[test]
fn a_surface_plate_is_the_ground_and_a_high_deck_is_not() {
    use opts_bevy::TileTerrain;
    use opts_core::formats::map::MapTerrain;
    let flat = || {
        let mut index = GroundIndex::default();
        let terrain = TileTerrain::new(
            MapTerrain {
                grid_cells: 60,
                heights: vec![0.0; 61 * 61],
            },
            5.0,
        );
        index.insert_tile((0, 0), (0.0, 0.0), Some(terrain), &[]);
        index
    };
    let mut index = flat();
    let plate =
        |z0: f32, z1: f32| Obb::new(Vec2::new(100.0, 100.0), Vec2::new(5.0, 5.0), 0.0, z0, z1);
    index.insert_plates((0, 0), [plate(0.25, 0.35)]);
    let h = index.height_at(100.0, 100.0, 0.0).expect("height");
    assert!((h - 0.35).abs() < 0.01, "on the plate: {h}");
    let outside = index.height_at(120.0, 100.0, 0.0).expect("height");
    assert!(outside.abs() < 0.01, "beside it: {outside}");
    let mut over = flat();
    over.insert_plates((0, 0), [plate(5.0, 6.0)]);
    let under = over.height_at(100.0, 100.0, 0.0).expect("height");
    assert!(under.abs() < 0.01, "under the deck: {under}");
    let on_deck = over.height_at(100.0, 100.0, 6.0).expect("height");
    assert!((on_deck - 6.0).abs() < 0.01, "on the deck: {on_deck}");
}

/// A sample tile of undulating terrain (60 cells of 5 m), in the tile's own metres.
fn sample_terrain() -> TileTerrain {
    let cell = 5.0_f32;
    let mut heights = Vec::with_capacity(61 * 61);
    for j in 0..61 {
        for i in 0..61 {
            let (x, y) = (i as f32 * cell, j as f32 * cell);
            heights.push(7.0 + 0.03 * x - 0.02 * y + 1.5 * (x * 0.07).sin() * (y * 0.05).cos());
        }
    }
    TileTerrain::new(
        opts_core::formats::map::MapTerrain {
            grid_cells: 60,
            heights,
        },
        cell,
    )
}

/// Before the terrain moved into `GroundElevationQuery`: both height services answer the same
/// bits for the same points of a resident tile (anchor at tile `(0, 0)`, so session and absolute
/// frames coincide).
#[test]
fn ground_index_and_elevation_query_agree_on_a_sample_tile() {
    let grid = TileGrid::Uniform { pitch: 300.0 };
    let index = GroundIndex::for_tile(
        (0, 0),
        (0.0, 0.0),
        Some(sample_terrain()),
        &[],
        (0, 0),
        grid.clone(),
    );
    let mut query = opts_bevy::GroundElevationQuery::new(grid);
    query.insert((0, 0), sample_terrain());
    let mut checked = 0;
    for i in 0..=120 {
        for j in 0..=120 {
            let (x, y) = (i as f32 * 2.5 + 0.1, j as f32 * 2.5 + 0.3);
            let a = index.terrain_at(x, y);
            let b = query.height_at(x, y);
            assert_eq!(a.map(f32::to_bits), b.map(f32::to_bits), "at ({x}, {y})");
            checked += 1;
        }
    }
    assert!(checked > 10_000);
    assert!(index.terrain_at(-1.0, 10.0).is_none(), "outside the tile");
}
