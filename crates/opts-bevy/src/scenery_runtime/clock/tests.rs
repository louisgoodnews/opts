//! Tests of the scenery clock and the host variable names.

#[test]
fn brightness_follows_the_sun() {
    let b = |alt: f32| {
        let mut c = super::SceneryClock::at(2026, 6, 29, 43_200.0);
        c.sun_alt_deg = alt;
        c.envir_brightness()
    };
    assert_eq!(b(-20.0), 0.0);
    assert_eq!(b(40.0), 1.0);
    assert!(b(0.0) > 0.0 && b(0.0) < 0.35, "{}", b(0.0));
    assert!(b(9.0) > 0.75 && b(9.0) < 0.85, "{}", b(9.0));
}

use super::*;

#[test]
fn day_of_year_counts_leap_days() {
    assert_eq!(SceneryClock::at(2026, 1, 1, 0.0).day_of_year(), 1);
    assert_eq!(SceneryClock::at(2026, 6, 29, 0.0).day_of_year(), 180);
    assert_eq!(SceneryClock::at(2024, 3, 1, 0.0).day_of_year(), 61);
}

#[test]
fn noon_is_day_and_late_evening_is_night() {
    let noon = SceneryClock::at(2026, 6, 29, 12.0 * 3600.0);
    assert!(noon.sun_alt_deg > 50.0 && !noon.night);
    let late = SceneryClock::at(2026, 6, 29, 23.0 * 3600.0);
    assert!(late.night);
    let mut c = noon.clone();
    c.advance(11.0 * 3600.0);
    assert!(c.night);
    assert!((c.time_of_day_s() - 23.0 * 3600.0).abs() < 1.0);
}

#[test]
fn the_scenery_clock_and_the_session_environment_agree_on_the_sun() {
    use crate::environment::{CalendarDate, SessionEnvironment};
    for (date, h) in [
        (CalendarDate::new(2026, 6, 29), 5.25),
        (CalendarDate::new(1994, 12, 31), 15.75),
        (CalendarDate::new(2024, 3, 1), 21.0),
    ] {
        let clock = SceneryClock::at(date.year, date.month, date.day, h * 3600.0);
        let env = SessionEnvironment::new(date, h * 3600.0, "");
        assert!((clock.sun_alt_deg - env.sun_alt_deg()).abs() < 1e-4);
        assert_eq!(clock.night, env.night());
        assert_eq!(clock.day_of_year(), date.day_of_year());
    }
}

#[test]
fn host_variables_are_matched_case_insensitively() {
    assert_eq!(HostVar::of("NightLightA"), HostVar::Night);
    assert_eq!(HostVar::of("TrafficLightPhase"), HostVar::Phase);
    assert_eq!(HostVar::of("red"), HostVar::Zero);
    assert_eq!(HostVar::of("Winterhalbjahr"), HostVar::WinterHalf);
    assert_eq!(HostVar::of("sommerhalbjahr"), HostVar::SummerHalf);
}
