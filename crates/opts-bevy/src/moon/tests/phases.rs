//! The phases of the moon: the principal phases, the fraction lit and its monthly course.

use super::*;

#[test]
fn eleven_principal_phases_are_within_ten_minutes() {
    // (year, month, day, hour, minute UTC, target elongation) from published tables.
    let table = [
        (2000, 1, 6, 18, 14, 0.0),
        (2000, 1, 21, 4, 40, 180.0),
        (2024, 1, 11, 11, 57, 0.0),
        (2024, 1, 18, 3, 53, 90.0),
        (2024, 1, 25, 17, 54, 180.0),
        (2024, 4, 8, 18, 21, 0.0),
        (2024, 6, 22, 1, 8, 180.0),
        (2022, 11, 8, 11, 2, 180.0),
        (2025, 3, 14, 6, 55, 180.0),
        (2025, 9, 21, 19, 54, 0.0),
        (2024, 1, 4, 3, 30, 270.0),
    ];
    for (y, mo, d, h, mi, target) in table {
        let reference = jd_of(y, mo, d, h, mi);
        // A target of 0 sits at the 360 wrap: solve on the signed difference.
        let found = solve(reference, target);
        let minutes = (found - reference) * 1440.0;
        assert!(
            minutes.abs() <= 10.0,
            "{y}-{mo:02}-{d:02} {h:02}:{mi:02}: {minutes:+.1} min"
        );
    }
}

#[test]
fn the_fraction_is_zero_at_new_one_at_full_half_at_quarters() {
    let at = |y, mo, d, h, mi, target| {
        let jd = solve(jd_of(y, mo, d, h, mi), target);
        let date = CalendarDate::from_days_since_epoch((jd - 2_440_587.5).floor() as i64);
        let secs = (jd - 2_440_587.5 - (jd - 2_440_587.5).floor()) * 86_400.0;
        moon_state(date, secs, 50.0)
    };
    // The moon's ecliptic latitude (up to 5°) keeps the new and full fractions a hair off 0 / 1.
    assert!(at(2024, 1, 11, 11, 57, 0.0).illuminated < 3e-3);
    assert!(at(2024, 1, 25, 17, 54, 180.0).illuminated > 0.997);
    let q = at(2024, 1, 18, 3, 53, 90.0);
    assert!((q.illuminated - 0.5).abs() < 1e-3 && q.waxing);
    let q = at(2024, 1, 4, 3, 30, 270.0);
    assert!((q.illuminated - 0.5).abs() < 1e-3 && !q.waxing);
}

#[test]
fn the_fraction_waxes_for_half_a_month_and_repeats() {
    let date = CalendarDate::new(2026, 10, 1);
    let k = |days: f64| {
        let jd = julian_day(date, 0.0) + days;
        let d = CalendarDate::from_days_since_epoch((jd - 2_440_587.5).floor() as i64);
        let s = (jd - 2_440_587.5).rem_euclid(1.0) * 86_400.0;
        moon_state(d, s, 50.0)
    };
    // Day-to-day fractions rise strictly through the waxing half.
    let mut prev: Option<f32> = None;
    let mut waxing_days = 0;
    for i in 0..40 {
        let s = k(f64::from(i));
        if s.waxing {
            if let Some(p) = prev {
                assert!(s.illuminated > p - 1e-6);
            }
            waxing_days += 1;
            prev = Some(s.illuminated);
        } else {
            prev = None;
        }
    }
    assert!((13..=17).contains(&waxing_days), "{waxing_days}");
    // A synodic month later the phase is almost the same.
    let (a, b) = (k(3.0), k(3.0 + SYNODIC_MONTH_DAYS));
    assert!((a.illuminated - b.illuminated).abs() < 0.05);
}
