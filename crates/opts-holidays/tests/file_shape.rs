//! Holiday file shapes as the map load meets them: keywords in any case, other sections that
//! are passed over, comment lines inside a record, and a school period with a bad end date.
//!
//! Weekdays are hand-computed: 2026-12-23 is a Wednesday, 2026-12-22 a Tuesday; 2024-02-29 a
//! Thursday; 2100-02-28 a Sunday and 2100-03-01 a Monday (2100 is not a leap year).

use opts_holidays::{weekday, DayKind, Holidays};

#[test]
fn keywords_match_in_any_case_and_fields_are_trimmed() {
    let h = Holidays::parse("[HOLIDAY]\n 20261224 \nHeiligabend\n");
    assert_eq!(h.public.len(), 1);
    assert_eq!(h.public[0].date, 20261224);
    assert_eq!(h.public[0].name, "Heiligabend");
}

#[test]
fn other_sections_are_passed_over_with_their_lines() {
    let h = Holidays::parse("[set_day_of_week]\n20261225\nFoo\n[holiday]\n20261001\nTag\n");
    assert_eq!(h.public.len(), 1, "only the holiday record counts");
    assert_eq!(h.public[0].date, 20261001);
    assert!(h.school.is_empty());
}

#[test]
fn comment_lines_inside_a_record_are_skipped() {
    let h = Holidays::parse("[holiday]\n....\n----\n20261225\nWeihnachten\n");
    assert_eq!(h.public.len(), 1);
    assert_eq!(h.public[0].date, 20261225);
    assert_eq!(h.public[0].name, "Weihnachten");
}

#[test]
fn a_school_period_with_a_bad_end_date_is_dropped_and_the_next_is_read() {
    let text = "[holidays]\n20260715\n2026073x\nSommer\n[holidays]\n20261223\n20261231\nWinter\n";
    let h = Holidays::parse(text);
    assert_eq!(h.school.len(), 1);
    assert_eq!(h.school[0].name, "Winter");
    assert_eq!((h.school[0].start, h.school[0].end), (20261223, 20261231));
}

#[test]
fn a_public_holiday_on_a_wednesday_selects_the_sunday_schedule() {
    let h = Holidays::parse("[holiday]\n20261223\nX\n");
    assert_eq!(weekday(20261223), 3, "Wednesday");
    assert_eq!(h.day_kind(20261223), DayKind::Sunday);
    assert_eq!(h.day_kind(20261222), DayKind::Weekday, "the Tuesday before");
}

#[test]
fn the_leap_day_of_2024_is_a_thursday() {
    assert_eq!(weekday(20240229), 4);
}

#[test]
fn a_century_year_that_is_not_a_leap_year_has_no_29th_of_february() {
    assert_eq!(weekday(21000228), 7, "Sunday");
    assert_eq!(weekday(21000301), 1, "Monday");
}

#[test]
fn an_empty_file_gives_no_holidays() {
    assert_eq!(Holidays::parse(""), Holidays::default());
}
