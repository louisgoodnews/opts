//! Unit tests of the element index (M-0105 deliverable 3). Written without running them.

use glam::Vec3;
use opts_core::formats::map::{MapObjectInstance, MapSplineInstance, MapTile};
use opts_traffic::course::{ElementCache, ElementIndex, ElementKind, ElementRef, Lookup};

fn spline(id: u64) -> MapSplineInstance {
    MapSplineInstance {
        flag: 0,
        sli_file: String::from("road.sli"),
        id_code: id,
        conn_id_1: 0,
        conn_id_2: 0,
        position: Vec3::ZERO,
        heading_deg: 0.0,
        length_m: 10.0,
        radius_m: 0.0,
        gradient_start: 0.0,
        name: String::new(),
    }
}

fn object(id: u64) -> MapObjectInstance {
    MapObjectInstance {
        flag: 0,
        sco_file: String::from("stop.sco"),
        id_code: id,
        position: Vec3::ZERO,
        heading_deg: 0.0,
        pitch_deg: None,
        roll_deg: None,
        string_vars: Vec::new(),
    }
}

fn tile_with(splines: &[u64], objects: &[u64]) -> MapTile {
    MapTile {
        splines: splines.iter().map(|&i| spline(i)).collect(),
        objects: objects.iter().map(|&i| object(i)).collect(),
        ..MapTile::default()
    }
}

#[test]
fn finds_splines_and_objects_on_their_tile() {
    let mut index = ElementIndex::new();
    let added = index.index_tile((1, 2), &tile_with(&[10], &[20]));
    assert_eq!(added, 2);
    assert_eq!(
        index.lookup(10, &[(1, 2)]),
        Lookup::Found(ElementRef {
            tile: (1, 2),
            kind: ElementKind::Spline
        })
    );
    assert_eq!(
        index.lookup(20, &[(1, 2)]),
        Lookup::Found(ElementRef {
            tile: (1, 2),
            kind: ElementKind::Object
        })
    );
}

#[test]
fn a_miss_on_unindexed_tiles_reports_them_and_loads_nothing() {
    let mut index = ElementIndex::new();
    index.index_tile((0, 0), &tile_with(&[1], &[]));
    assert_eq!(
        index.lookup(99, &[(0, 0), (2, 1), (1, 1), (2, 1)]),
        Lookup::NotIndexed {
            unindexed_tiles: vec![(1, 1), (2, 1)]
        }
    );
    assert_eq!(index.missing(&[(0, 0), (2, 1)]), vec![(2, 1)]);
}

#[test]
fn a_miss_with_every_tile_indexed_is_absent() {
    let mut index = ElementIndex::new();
    index.index_tile((0, 0), &tile_with(&[1], &[]));
    assert_eq!(index.lookup(99, &[(0, 0)]), Lookup::Absent);
}

#[test]
fn indexing_a_tile_twice_adds_nothing() {
    let mut index = ElementIndex::new();
    assert_eq!(index.index_tile((0, 0), &tile_with(&[1], &[])), 1);
    assert_eq!(index.index_tile((0, 0), &tile_with(&[1], &[])), 0);
    assert_eq!(index.tile_count(), 1);
    assert!(index.duplicates().is_empty());
}

#[test]
fn first_placement_wins_and_the_second_is_reported() {
    let mut index = ElementIndex::new();
    index.index_tile((0, 0), &tile_with(&[7], &[]));
    index.index_tile((1, 0), &tile_with(&[7], &[]));
    assert_eq!(
        index.lookup(7, &[]),
        Lookup::Found(ElementRef {
            tile: (0, 0),
            kind: ElementKind::Spline
        })
    );
    assert_eq!(index.duplicates().len(), 1);
    assert_eq!(index.duplicates()[0].0, 7);
}

#[test]
fn cache_starts_empty_when_the_chrono_state_changes() {
    let mut cache = ElementCache::default();
    cache.get(1).index_tile((0, 0), &tile_with(&[5], &[]));
    assert_eq!(cache.get(1).len(), 1);
    assert_eq!(cache.get(2).len(), 0);
    cache.clear();
    assert!(cache.get(2).is_empty());
}
