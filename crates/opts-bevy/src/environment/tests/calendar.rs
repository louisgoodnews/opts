//! The calendar and the clock: day numbers, weekdays, parsing, the wrap and the moon.

use super::*;

#[test]
fn dates_round_trip_through_day_numbers() {
    for d in [
        CalendarDate::new(1970, 1, 1),
        CalendarDate::new(1994, 12, 31),
        CalendarDate::new(2000, 2, 29),
        CalendarDate::new(1900, 3, 1),
        CalendarDate::new(2026, 10, 17),
    ] {
        assert_eq!(CalendarDate::from_days_since_epoch(d.days_since_epoch()), d);
    }
    assert_eq!(CalendarDate::new(1970, 1, 1).days_since_epoch(), 0);
    assert_eq!(
        CalendarDate::new(1994, 12, 31).add_days(1),
        CalendarDate::new(1995, 1, 1)
    );
    assert_eq!(
        CalendarDate::new(2024, 2, 28).add_days(1),
        CalendarDate::new(2024, 2, 29)
    );
}

#[test]
fn weekdays_and_day_bits() {
    // 2026-10-17 is a Saturday, 1989-10-09 a Monday, 1994-12-31 a Saturday.
    assert_eq!(CalendarDate::new(2026, 10, 17).weekday(), 6);
    assert_eq!(CalendarDate::new(1989, 10, 9).weekday(), 1);
    assert_eq!(CalendarDate::new(2026, 10, 18).day_bit(), day::SUNDAY);
    assert_eq!(CalendarDate::new(1994, 12, 31).day_bit(), day::SATURDAY);
    assert_eq!(CalendarDate::new(2026, 10, 14).day_bit(), day::WEEKDAY);
}

#[test]
fn parsing_and_clamping() {
    assert_eq!(
        CalendarDate::parse("1994-12-31"),
        Some(CalendarDate::new(1994, 12, 31))
    );
    assert_eq!(CalendarDate::parse("1994-02-30"), None);
    assert_eq!(CalendarDate::parse("x"), None);
    assert_eq!(
        CalendarDate::clamped(1993, 2, 29),
        CalendarDate::new(1993, 2, 28)
    );
    assert_eq!(CalendarDate::new(1994, 3, 1).to_string(), "1994-03-01");
}

#[test]
fn the_clock_wraps_and_rolls_the_date_over() {
    let mut env = SessionEnvironment::new(CalendarDate::new(1994, 12, 31), 23.5 * 3600.0, "");
    let rev = env.revision();
    env.advance(3600.0);
    assert_eq!(env.date(), CalendarDate::new(1995, 1, 1));
    assert!((env.seconds_of_day() - 1800.0).abs() < 1e-6);
    assert!((env.hour() - 0.5).abs() < 1e-6);
    assert_eq!(env.revision(), rev, "running is not a jump");
    assert_eq!(env.map_date(), CalendarDate::new(1994, 12, 31));
    assert_eq!(env.clock_text(), "00:30:00");
}

#[test]
fn the_moon_follows_the_clock() {
    let mut env = SessionEnvironment::new(CalendarDate::new(2026, 10, 17), 18.0 * 3600.0, "");
    let before = *env.moon();
    env.advance(4.0 * 3600.0);
    assert_ne!(before.elevation_deg, env.moon().elevation_deg);
    assert!((before.illuminated - env.moon().illuminated).abs() < 0.02);
}
