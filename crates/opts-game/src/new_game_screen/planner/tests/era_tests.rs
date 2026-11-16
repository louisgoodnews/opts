//! Era tests of the planner: the default date from the map's years, and the era hint.

use super::*;

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
