//! Unit tests for `super` (`drive_history.rs`).

use super::*;

fn rec(map: &str, t: i64) -> DriveRecord {
    DriveRecord {
        map: map.into(),
        last_driven: t,
        ..Default::default()
    }
}

#[test]
fn the_same_choices_move_to_the_front_and_are_listed_once() {
    let mut h = DriveHistory::default();
    h.record(rec("A", 1));
    h.record(rec("B", 2));
    h.record(rec("A", 3));
    assert_eq!(h.recents.len(), 2);
    assert_eq!(
        h.last().map(|r| (r.map.as_str(), r.last_driven)),
        Some(("A", 3))
    );
}

#[test]
fn only_the_newest_drives_are_kept() {
    let mut h = DriveHistory::default();
    for i in 0..10 {
        h.record(rec(&format!("m{i}"), i));
    }
    assert_eq!(h.recents.len(), DriveHistory::MAX_RECENTS);
    assert_eq!(h.recents[0].map, "m9");
}

#[test]
fn a_setup_survives_a_round_trip_through_a_record() {
    let mut setup = NewGameSetup {
        selected_map: "Grundorf".into(),
        hour: 17,
        ..NewGameSetup::default()
    };
    setup.custom_setvars.insert("Klima".into(), 2);
    let record = DriveRecord::from_setup(&setup, 42);
    let json = serde_json::to_string(&record).expect("serialises");
    let back: DriveRecord = serde_json::from_str(&json).expect("parses");
    let mut other = NewGameSetup::default();
    back.apply_to(&mut other);
    assert_eq!(other.selected_map, "Grundorf");
    assert_eq!(other.hour, 17);
    assert_eq!(other.custom_setvars.get("Klima"), Some(&2));
}
