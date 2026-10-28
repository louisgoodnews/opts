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

#[test]
fn the_date_stays_valid_when_a_month_changes() {
    let mut s = setup();
    s.year = 1989;
    s.month = 1;
    s.day = 31;
    step_field(&mut s, Field::Month, 1, false);
    assert_eq!((s.month, s.day), (2, 28));
    s.year = 1988;
    s.month = 1;
    s.day = 31;
    step_field(&mut s, Field::Month, 1, false);
    assert_eq!((s.month, s.day), (2, 29));
    s.month = 12;
    step_field(&mut s, Field::Month, 1, false);
    assert_eq!(s.month, 1);
}

#[test]
fn days_hours_and_minutes_wrap_and_years_clamp() {
    let mut s = setup();
    s.month = 4;
    s.day = 30;
    step_field(&mut s, Field::Day, 1, false);
    assert_eq!(s.day, 1);
    s.hour = 23;
    step_field(&mut s, Field::Hour, 1, false);
    assert_eq!(s.hour, 0);
    s.minute = 55;
    step_field(&mut s, Field::Minute, 1, false);
    assert_eq!(s.minute, 0);
    s.minute = 3;
    step_field(&mut s, Field::Minute, 1, true);
    assert_eq!(s.minute, 4);
    s.minute = 0;
    step_field(&mut s, Field::Minute, -1, true);
    assert_eq!(s.minute, 59);
    s.year = 2100;
    step_field(&mut s, Field::Year, 5, false);
    assert_eq!(s.year, 2100);
    s.year = 1900;
    step_field(&mut s, Field::Year, -5, false);
    assert_eq!(s.year, 1900);
}

#[test]
fn the_default_date_shifts_today_and_clamps_into_the_years() {
    let spandau = MapEra {
        years: Some((1986, 1994)),
        real_year_offset: -25,
        ..MapEra::default()
    };
    // 2026-10-17 − 25 years = 2001-10-17, after the map's last year.
    assert_eq!(
        default_map_date(CalendarDate::new(2026, 10, 17), &spandau),
        CalendarDate::new(1994, 12, 31)
    );
    // 2012-03-04 − 25 = 1987-03-04, inside the years.
    assert_eq!(
        default_map_date(CalendarDate::new(2012, 3, 4), &spandau),
        CalendarDate::new(1987, 3, 4)
    );
    // Before the first year: 1 January.
    assert_eq!(
        default_map_date(CalendarDate::new(2005, 6, 1), &spandau),
        CalendarDate::new(1986, 1, 1)
    );
    // A map without [years] keeps the shifted date; 29 February becomes 28 February.
    let open = MapEra {
        years: None,
        real_year_offset: -1,
        ..MapEra::default()
    };
    assert_eq!(
        default_map_date(CalendarDate::new(2024, 2, 29), &open),
        CalendarDate::new(2023, 2, 28)
    );
    assert_eq!(
        default_map_date(CalendarDate::new(2026, 10, 17), &MapEra::default()),
        CalendarDate::new(2026, 10, 17)
    );
}

#[test]
fn the_era_comes_from_global_cfg() {
    let m = opts_core::formats::map::MapManifest::from_text(
        "[years]\n1994\n1986\n\n[realyearoffset]\n-25\n",
    )
    .expect("parse");
    assert_eq!(
        MapEra::of(&m),
        MapEra {
            years: Some((1986, 1994)),
            real_year_offset: -25,
            start_clock: None
        }
    );
}

#[test]
fn the_start_clock_is_the_maps_saved_time_else_now() {
    let with = MapEra {
        start_clock: Some((21, 33)),
        ..MapEra::default()
    };
    assert_eq!(
        default_start_clock(&with, (9, 5)),
        ((21, 33), TimeSource::Map)
    );
    assert_eq!(
        default_start_clock(&MapEra::default(), (9, 5)),
        ((9, 5), TimeSource::Now)
    );
}

#[test]
fn the_era_hint_appears_only_outside_the_years_of_the_map() {
    assert_eq!(era_hint("1986 - 1994", 2020), Some((1986, 1994)));
    assert_eq!(era_hint("1986 - 1994", 1990), None);
    assert_eq!(era_hint("", 2020), None);
    assert_eq!(era_hint("ab", 2020), None);
}
