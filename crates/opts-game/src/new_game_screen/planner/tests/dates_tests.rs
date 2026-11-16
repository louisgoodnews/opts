//! Date and time tests of the planner: the default date, the wrap of days, hours and minutes.

use super::*;

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

