//! Unit tests for `super` (`planner.rs`).

use super::*;
use crate::new_game_screen::types::{MapItem, VehicleItem};

fn catalogue() -> GameCatalogue {
    GameCatalogue {
        maps: vec![MapItem {
            id: "m".into(),
            name: "m".into(),
            friendly_name: "Berlin".into(),
            description: String::new(),
            tile_count: 4,
            standard_depot: String::new(),
            years: "1986 - 1994".into(),
            entry_points: vec!["A".into(), "B".into()],
        }],
        vehicles: vec![VehicleItem::named("v", "MAN", "SD77")],
    }
}

fn setup() -> NewGameSetup {
    NewGameSetup {
        selected_map: "Berlin".into(),
        selected_entry_point: "A".into(),
        selected_vehicle: "MAN SD77".into(),
        ..NewGameSetup::default()
    }
}

#[test]
fn steps_walk_forwards_and_backwards() {
    assert_eq!(PlannerStep::Where.prev(), None);
    assert_eq!(PlannerStep::Where.next(), Some(PlannerStep::What));
    assert_eq!(PlannerStep::Review.next(), None);
    assert_eq!(PlannerStep::Review.prev(), Some(PlannerStep::Trip));
}

#[test]
fn validity_names_the_first_problem_in_step_order() {
    let c = catalogue();
    assert_eq!(validate(&setup(), &c), Ok(()));
    let mut s = setup();
    s.selected_map = "Nowhere".into();
    s.selected_vehicle = "Nothing".into();
    assert_eq!(validate(&s, &c), Err(Invalid::NoMap));
    let mut s = setup();
    s.selected_entry_point = "Z".into();
    assert_eq!(validate(&s, &c), Err(Invalid::NoStartPoint));
    let mut s = setup();
    s.selected_vehicle = "x".into();
    assert_eq!(validate(&s, &c), Err(Invalid::NoVehicle));
}

#[test]
fn an_invalid_selection_is_repaired_from_the_catalogue() {
    let c = catalogue();
    let mut s = NewGameSetup::default();
    ensure_valid(&mut s, &c);
    assert_eq!(validate(&s, &c), Ok(()));
    assert_eq!(s.selected_map, "Berlin");
    assert_eq!(s.selected_entry_point, "A");
    let mut keep = setup();
    keep.selected_entry_point = "B".into();
    ensure_valid(&mut keep, &c);
    assert_eq!(
        keep.selected_entry_point, "B",
        "a valid choice is not touched"
    );
}

#[test]
fn a_map_without_start_points_needs_none() {
    let mut c = catalogue();
    c.maps[0].entry_points.clear();
    let mut s = setup();
    s.selected_entry_point = String::new();
    assert_eq!(validate(&s, &c), Ok(()));
}

#[test]
fn now_is_rounded_to_five_minutes_and_wraps() {
    assert_eq!(round_to_five(8, 32), (8, 30));
    assert_eq!(round_to_five(8, 33), (8, 35));
    assert_eq!(round_to_five(23, 58), (0, 0));
    let now = &QUICK_TIMES[0];
    assert_eq!(quick_time_of(now, (14, 17)), (14, 15));
    assert_eq!(quick_time_of(&QUICK_TIMES[3], (1, 1)), (18, 0));
}

#[test]
fn exactly_one_chip_matches_a_fixed_time() {
    let mut s = setup();
    s.hour = 12;
    s.minute = 0;
    assert_eq!(matching_quick_time(&s), Some(2));
    s.minute = 5;
    assert_eq!(matching_quick_time(&s), None);
}

mod dates_tests;
mod era_tests;
