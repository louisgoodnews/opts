//! Unit tests for `super` (`calendar.rs`).

use super::*;

#[test]
fn the_date_stays_valid_when_a_month_changes() {
    let mut d = CalendarDate::new(1989, 1, 31);
    step_date(&mut d, Field::Month, 1);
    assert_eq!(d, CalendarDate::new(1989, 2, 28));
    let mut d = CalendarDate::new(1988, 1, 31);
    step_date(&mut d, Field::Month, 1);
    assert_eq!(d, CalendarDate::new(1988, 2, 29));
    let mut d = CalendarDate::new(1988, 12, 1);
    step_date(&mut d, Field::Month, 1);
    assert_eq!(d.month, 1);
}

#[test]
fn days_wrap_years_clamp_and_time_wraps() {
    let mut d = CalendarDate::new(1990, 4, 30);
    step_date(&mut d, Field::Day, 1);
    assert_eq!(d.day, 1);
    let mut d = CalendarDate::new(2100, 4, 1);
    step_date(&mut d, Field::Year, 5);
    assert_eq!(d.year, 2100);
    let (mut h, mut m) = (23, 55);
    step_time(&mut h, &mut m, Field::Hour, 1, false);
    step_time(&mut h, &mut m, Field::Minute, 1, false);
    assert_eq!((h, m), (0, 0));
    step_time(&mut h, &mut m, Field::Minute, -1, true);
    assert_eq!(m, 59);
    let (mut h, mut m) = (8, 33);
    step_time(&mut h, &mut m, Field::Minute, 1, false);
    assert_eq!((h, m), (8, 35));
}
