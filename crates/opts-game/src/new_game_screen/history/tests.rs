//! Unit tests for `super` (`history.rs`).

use super::*;
use crate::new_game_screen::types::{MapItem, VehicleItem};

fn cat() -> GameCatalogue {
    GameCatalogue {
        maps: vec![MapItem {
            id: "b".into(),
            name: "b".into(),
            friendly_name: "Berlin".into(),
            description: String::new(),
            tile_count: 1,
            standard_depot: String::new(),
            years: String::new(),
            entry_points: vec!["A".into(), "B".into()],
        }],
        vehicles: vec![VehicleItem::named("v", "MAN", "SD77")],
    }
}

fn rec(map: &str, vehicle: &str) -> DriveRecord {
    DriveRecord {
        map: map.into(),
        entry_point: "B".into(),
        vehicle: vehicle.into(),
        no_vehicle: false,
        livery: "Beige".into(),
        weather_preset: "Rainy Afternoon".into(),
        year: 1990,
        month: 5,
        day: 6,
        hour: 18,
        minute: 0,
        setvars: vec![("x".into(), 1)],
        last_driven: 5,
    }
}

#[test]
fn the_last_drive_is_applied_once_per_session() {
    let mut h = PlannerHistory::at(None);
    h.history.record(rec("Berlin", "MAN SD77"));
    let mut s = NewGameSetup::default();
    let notes = h.apply_last(&mut s, &cat());
    assert!(notes.is_empty());
    assert_eq!(s.selected_map, "Berlin");
    assert_eq!(s.selected_entry_point, "B");
    assert_eq!(s.weather_preset, "Rainy Afternoon");
    assert_eq!((s.hour, s.minute), (18, 0));
    s.hour = 3;
    assert!(h.apply_last(&mut s, &cat()).is_empty());
    assert_eq!(s.hour, 3, "a second call does not overwrite later choices");
}

#[test]
fn a_vanished_map_or_vehicle_becomes_a_note_and_keeps_the_default() {
    let mut h = PlannerHistory::at(None);
    h.history.record(rec("Gone", "Gone too"));
    let mut s = NewGameSetup {
        selected_map: "Berlin".into(),
        ..NewGameSetup::default()
    };
    let notes = h.apply_last(&mut s, &cat());
    assert_eq!(notes, vec![Note::MapMissing, Note::VehicleMissing]);
    assert_eq!(s.selected_map, "Berlin");
}

#[test]
fn nothing_is_consumed_while_the_catalogue_is_empty() {
    let mut h = PlannerHistory::at(None);
    h.history.record(rec("Berlin", "MAN SD77"));
    let mut s = NewGameSetup::default();
    assert!(h.apply_last(&mut s, &GameCatalogue::default()).is_empty());
    assert!(!h.applied);
}

#[test]
fn a_drive_is_recorded_and_written_to_the_file() {
    let dir = std::env::temp_dir().join(format!("opts_hist_{}", std::process::id()));
    let path = dir.join("drive_history.json");
    let _ = std::fs::remove_dir_all(&dir);
    let mut h = PlannerHistory::at(Some(path.clone()));
    let s = NewGameSetup {
        selected_map: "Berlin".into(),
        hour: 7,
        ..NewGameSetup::default()
    };
    h.record_drive(&s, 42).expect("write");
    h.set_favourite(false, "b", true).expect("write");
    let back = DriveHistory::load_from(&path);
    assert_eq!(
        back.last().map(|r| (r.map.as_str(), r.hour, r.last_driven)),
        Some(("Berlin", 7, 42))
    );
    assert_eq!(back.favourite_maps, vec!["b".to_string()]);
    h.set_favourite(false, "b", false).expect("write");
    assert!(DriveHistory::load_from(&path).favourite_maps.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}
