//! Holiday calendar: parsing a holiday file and the day kind it selects.
//!
//! The fixture is synthetic, not a game file. Weekdays are hand-computed from the calendar
//! (2026-12-25 is a Friday; 2026-12-19 and 12-20 a Saturday and a Sunday).

use opts_holidays::{weekday, DayKind, Holidays};

const FIXTURE: &str = "-----------------2026------------------------
[holiday]
20261225
Weihnachten
....
[holidays]
20260715
20260731
Sommer
1
[holiday]
20261340
Falsch
[holiday]
20261226
Zweiter Feiertag
trailing comment
";

fn fixture() -> Holidays {
    Holidays::parse(FIXTURE)
}

#[test]
fn public_and_school_records_are_read_and_malformed_dates_skipped() {
    let h = fixture();
    let names: Vec<(u32, &str)> = h.public.iter().map(|p| (p.date, p.name.as_str())).collect();
    // The month 13 record is dropped; the trailing comment lines are not names.
    assert_eq!(names, vec![(20261225, "Weihnachten"), (20261226, "Zweiter Feiertag")]);
    assert_eq!(h.school.len(), 1);
    assert_eq!(
        (h.school[0].start, h.school[0].end, h.school[0].name.as_str()),
        (20260715, 20260731, "Sommer")
    );
}

#[test]
fn school_periods_include_both_ends() {
    let h = fixture();
    assert!(h.is_school(20260715));
    assert!(h.is_school(20260731));
    assert!(!h.is_school(20260714));
    assert!(!h.is_school(20260801));
}

#[test]
fn weekdays_are_hand_computed() {
    assert_eq!(weekday(19700101), 4, "1970-01-01 was a Thursday");
    assert_eq!(weekday(20000229), 2, "2000-02-29 was a Tuesday");
    assert_eq!(weekday(20261224), 4, "Thursday");
    assert_eq!(weekday(20261225), 5, "Friday");
    assert_eq!(weekday(20261219), 6, "Saturday");
    assert_eq!(weekday(20261220), 7, "Sunday");
}

#[test]
fn day_kind_follows_the_weekday_unless_a_public_holiday() {
    let h = fixture();
    assert_eq!(h.day_kind(20261224), DayKind::Weekday);
    assert_eq!(h.day_kind(20261219), DayKind::Saturday);
    assert_eq!(h.day_kind(20261220), DayKind::Sunday);
    // A public holiday on a Friday and on a Saturday selects the Sunday schedule.
    assert_eq!(h.day_kind(20261225), DayKind::Sunday);
    assert_eq!(h.day_kind(20261226), DayKind::Sunday);
}

#[test]
fn a_map_without_a_holiday_file_has_only_weekdays_and_weekends() {
    let h = Holidays::default();
    assert_eq!(h.day_kind(20261225), DayKind::Weekday);
    assert_eq!(h.day_kind(20261219), DayKind::Saturday);
    assert!(!h.is_school(20260715));
}
