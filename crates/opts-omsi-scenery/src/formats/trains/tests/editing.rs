//! Tests of the editing of a train formation: adding, removing and moving vehicles.

use super::*;

fn train(files: &[&str]) -> TrainFormation {
    let mut f = TrainFormation::custom("t");
    for file in files {
        f.add_file(file);
    }
    f
}

fn order(f: &TrainFormation) -> Vec<&str> {
    f.segments.iter().map(|s| s.vehicle.as_str()).collect()
}

#[test]
fn add_appends_forward_cars_and_returns_the_index() {
    let mut f = TrainFormation::custom("t");
    assert_eq!(f.add_file("a.ovh"), 0);
    assert_eq!(f.add_file("b.ovh"), 1);
    assert_eq!(f.segments[1].orientation, Orientation::Forward);
    assert_eq!(f.segments[1].powered, None);
}

#[test]
fn remove_and_move_keep_the_train_well_formed() {
    let mut f = train(&["a", "b", "c", "d"]);
    assert!(f.remove_car(1));
    assert_eq!(order(&f), ["a", "c", "d"]);
    assert!(!f.remove_car(9));
    assert_eq!(f.move_car(2, -1), Some(1));
    assert_eq!(order(&f), ["a", "d", "c"]);
    assert_eq!(f.move_car(0, -1), None, "cannot leave the front");
    assert_eq!(f.move_car(2, 1), None, "cannot leave the rear");
    assert_eq!(f.move_car(7, 1), None);
    assert_eq!(f.move_car(0, 2), Some(2));
    assert_eq!(order(&f), ["d", "c", "a"]);
    assert_eq!(f.len(), 3);
}

#[test]
fn turning_twice_restores_and_missing_cars_are_refused() {
    let mut f = train(&["a", "b"]);
    assert!(f.turn_car(1));
    assert_eq!(f.segments[1].orientation, Orientation::Backward);
    assert!(f.turn_car(1));
    assert_eq!(f.segments[1].orientation, Orientation::Forward);
    assert!(!f.turn_car(5));
}

#[test]
fn the_power_override_cycles_through_three_states() {
    let mut f = train(&["a", "b"]);
    assert_eq!(f.cycle_powered(1), Some(Some(true)));
    assert_eq!(f.cycle_powered(1), Some(Some(false)));
    assert_eq!(f.cycle_powered(1), Some(None));
    assert_eq!(f.cycle_powered(4), None);
}

#[test]
fn suggested_names_use_the_first_car_and_the_count() {
    assert_eq!(TrainFormation::custom("x").suggested_name(), "Formation");
    assert_eq!(
        train(&["vehicles\\BR218\\BR218_OR.ovh"]).suggested_name(),
        "BR218_OR"
    );
    assert_eq!(
        train(&["vehicles\\BR218\\BR218_OR.ovh", "w.ovh", "w.ovh"]).suggested_name(),
        "BR218_OR + 2"
    );
    assert_eq!(
        train(&["Vehicles/A/x.bus", "y.bus"]).suggested_name(),
        "x + 1"
    );
}
