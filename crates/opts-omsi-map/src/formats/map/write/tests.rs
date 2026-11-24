//! Unit tests for `super` (`write.rs`).

use glam::Vec3;

use super::*;
use crate::formats::map::{MapEntryPoint, MapGroundTex, MapSeason, MapTileEntry};

fn manifest() -> MapManifest {
    MapManifest {
        name: "OPTS Scenery Benchmark".into(),
        friendly_name: Some("Benchmark".into()),
        description: vec!["Generated map.".into(), "Second line".into()],
        version: Some(14),
        next_id_code: Some(3),
        money_system: Some(r"Money\DM\DM.cfg".into()),
        ticket_pack: None,
        standard_depot: Some("Depot".into()),
        years: Some((1990, 2010)),
        real_year_offset: Some(-3),
        repair_time_min: Some(2.5),
        world_coordinates: true,
        real_rail: false,
        dyn_helper_active: true,
        tiles: vec![MapTileEntry {
            x: 0,
            y: -1,
            file_name: "tile_0_-1.map".into(),
        }],
        entry_points: vec![MapEntryPoint {
            id: 0,
            id_code: 1,
            position: Vec3::new(20.0, 0.0, 20.0),
            direction: Vec3::new(0.0, 0.25, 1.0),
            name: "Benchmark".into(),
        }],
        ground_textures: vec![MapGroundTex {
            ground_texture: r"texture\grass.dds".into(),
            detail_texture: r"texture\detail.dds".into(),
            params: [1.0, 0.5, 30.0],
        }],
        seasons: vec![MapSeason {
            season_id: 3,
            start_day: 330,
            end_day: 60,
        }],
        traffic_density_road: vec![(0.0, 0.1), (8.5, 1.0)],
        traffic_density_passenger: vec![(12.0, 0.75)],
    }
}

fn object(strings: &[&str]) -> MapObjectInstance {
    MapObjectInstance {
        flag: 0,
        sco_file: r"Sceneryobjects\OPTS_Test\box.sco".into(),
        id_code: 2,
        position: Vec3::new(150.0, 150.25, -0.5),
        heading_deg: 37.0,
        pitch_deg: Some(1.5),
        roll_deg: Some(-2.0),
        string_vars: strings.iter().map(|s| (*s).to_string()).collect(),
    }
}

#[test]
fn manifest_round_trips() {
    let m = manifest();
    let text = m.to_text().unwrap();
    assert_eq!(MapManifest::from_text(&text).unwrap(), m);
}

#[test]
fn minimal_manifest_round_trips() {
    let m = MapManifest {
        name: "x".into(),
        ..MapManifest::default()
    };
    assert_eq!(MapManifest::from_text(&m.to_text().unwrap()).unwrap(), m);
}

#[test]
fn tile_objects_round_trip() {
    let tile = MapTile {
        version: Some(14),
        objects: vec![object(&["OPTS-A1", "OPTS A2 with spaces"]), object(&[])],
        ..MapTile::default()
    };
    let text = tile.to_text().unwrap();
    assert_eq!(MapTile::from_text(&text).unwrap(), tile);
}

/// Blank and comment-like string values are read line by line, so they round-trip.
#[test]
fn blank_and_comment_like_strings_round_trip() {
    let tile = MapTile {
        version: Some(14),
        objects: vec![object(&["", "// not a comment", "#1"]), object(&["x"])],
        ..MapTile::default()
    };
    let text = tile.to_text().unwrap();
    assert_eq!(MapTile::from_text(&text).unwrap(), tile);
}

#[test]
fn legacy_tile_round_trips_without_pitch() {
    let mut obj = object(&["a"]);
    obj.pitch_deg = None;
    obj.roll_deg = None;
    let tile = MapTile {
        version: Some(10),
        objects: vec![obj],
        ..MapTile::default()
    };
    assert_eq!(MapTile::from_text(&tile.to_text().unwrap()).unwrap(), tile);
}

#[test]
fn unwritable_strings_are_rejected() {
    for bad in [" padded", "two\nlines", "[keyword]", "# padded "] {
        let tile = MapTile {
            version: Some(14),
            objects: vec![object(&[bad])],
            ..MapTile::default()
        };
        assert!(
            matches!(tile.to_text(), Err(MapWriteError::UnwritableLine { .. })),
            "{bad:?} must be rejected"
        );
    }
}

#[test]
fn splines_are_not_written() {
    let tile = MapTile {
        has_terrain: true,
        ..MapTile::default()
    };
    assert!(matches!(
        tile.to_text(),
        Err(MapWriteError::UnsupportedTileContent(_))
    ));
}
