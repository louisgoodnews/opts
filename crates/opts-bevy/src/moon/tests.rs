//! Unit tests for `super` (`moon.rs`).

use super::*;

/// Julian day of a UTC date and time.
fn jd_of(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> f64 {
    julian_day(
        CalendarDate::new(y, mo, d),
        f64::from(h) * 3600.0 + f64::from(mi) * 60.0,
    )
}

/// Solves for the instant near `reference` at which the elongation passes `target` degrees.
fn solve(reference: f64, target: f64) -> f64 {
    let f = |jd: f64| {
        let mut d = lunar_elongation_deg(jd) - target;
        while d > 180.0 {
            d -= 360.0;
        }
        while d < -180.0 {
            d += 360.0;
        }
        d
    };
    let (mut lo, mut hi) = (reference - 1.0, reference + 1.0);
    assert!(f(lo) < 0.0 && f(hi) > 0.0, "bracket");
    for _ in 0..60 {
        let mid = 0.5 * (lo + hi);
        if f(mid) < 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

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

#[test]
fn the_moon_is_with_the_sun_at_new_and_opposite_at_full() {
    let sun_at = |date: CalendarDate, secs: f64| {
        crate::environment::sun_position((secs / 3600.0) as f32, date.day_of_year() as f32, 50.0)
    };
    for (y, mo, d, h, mi, target, near) in [
        (2024, 1, 11, 11, 57, 0.0, true),
        (2024, 1, 25, 17, 54, 180.0, false),
    ] {
        let jd = solve(jd_of(y, mo, d, h, mi), target);
        let date = CalendarDate::from_days_since_epoch((jd - 2_440_587.5).floor() as i64);
        let secs = (jd - 2_440_587.5).rem_euclid(1.0) * 86_400.0;
        let moon = moon_state(date, secs, 50.0);
        let (sun_el, sun_az) = sun_at(date, secs);
        if near {
            assert!((moon.elevation_deg - sun_el).abs() < 6.0);
            let mut daz = (moon.azimuth_deg - sun_az).abs();
            if daz > 180.0 {
                daz = 360.0 - daz;
            }
            assert!(daz < 15.0 || sun_el.abs() > 60.0, "{daz}");
        } else {
            assert!((moon.elevation_deg + sun_el).abs() < 8.0, "{sun_el}");
        }
    }
}

#[test]
fn sky_position_sample_of_the_design() {
    let date = CalendarDate::new(2026, 10, 17);
    let night = moon_state(date, 22.0 * 3600.0, 50.0);
    assert!((night.elevation_deg + 4.9).abs() < 1.5, "{night:?}");
    assert!((night.azimuth_deg - 235.0).abs() < 3.0, "{night:?}");
    assert!((night.illuminated - 0.43).abs() < 0.03, "{night:?}");
    assert!(night.waxing);
    let eve = moon_state(date, 18.0 * 3600.0, 50.0);
    assert!((eve.elevation_deg - 13.6).abs() < 1.5, "{eve:?}");
    assert!((eve.azimuth_deg - 186.0).abs() < 3.0, "{eve:?}");
}

#[test]
fn angles_are_in_range_and_the_model_is_deterministic() {
    for i in 0..200 {
        let date = CalendarDate::new(2026, 1, 1).add_days(i * 7);
        let secs = f64::from(i as i32 % 24) * 3600.0;
        let a = moon_state(date, secs, 50.0);
        assert_eq!(a, moon_state(date, secs, 50.0));
        assert!((0.0..360.0).contains(&a.azimuth_deg));
        assert!((-90.0..=90.0).contains(&a.elevation_deg));
        assert!((0.0..=1.0).contains(&a.illuminated));
        assert!((0.0..=180.0).contains(&a.phase_angle_deg));
        assert!((-180.0..=180.0).contains(&a.bright_limb_deg));
        assert!((0.0..30.0).contains(&a.age_days));
    }
}

#[test]
fn the_bright_limb_points_towards_the_sun_in_the_sky() {
    // A waxing evening moon in the south-west is lit on its right (sun below the horizon to
    // the west): the limb angle is near 0 (screen right) to within a quarter turn.
    let s = moon_state(CalendarDate::new(2026, 10, 17), 18.0 * 3600.0, 50.0);
    assert!(s.waxing);
    assert!(s.bright_limb_deg.abs() < 60.0, "{s:?}");
}

#[test]
fn allen_phase_function_matches_the_published_values() {
    assert!((allen_phase_function(0.0) - 1.0).abs() < 1e-6);
    assert!((allen_phase_function(60.0) - 0.23).abs() < 0.03);
    assert!((allen_phase_function(90.0) - 0.09).abs() < 0.02);
    assert!((allen_phase_function(120.0) - 0.026).abs() < 0.01);
}

fn state(el: f32, k: f32) -> MoonState {
    MoonState {
        elevation_deg: el,
        azimuth_deg: 180.0,
        elongation_deg: 0.0,
        illuminated: k,
        phase_angle_deg: 180.0 - (1.0 - 2.0 * k).acos().to_degrees(),
        bright_limb_deg: 0.0,
        waxing: true,
        age_days: 0.0,
    }
}

#[test]
fn moonlight_is_zero_below_the_horizon_by_day_and_at_new_moon() {
    assert_eq!(moon_illuminance(&state(-5.0, 1.0), -30.0, 1.0), 0.0);
    assert_eq!(moon_illuminance(&state(40.0, 1.0), 20.0, 1.0), 0.0);
    assert_eq!(moon_illuminance(&state(40.0, 0.0), -30.0, 1.0), 0.0);
    assert_eq!(moon_illuminance(&state(40.0, 1.0), -30.0, 0.0), 0.0);
    let full = moon_illuminance(&state(60.0, 1.0), -30.0, 1.0);
    assert!(full > 0.8 * MOON_PEAK_LX && full <= MOON_PEAK_LX, "{full}");
}

#[test]
fn moonlight_grows_with_phase_and_height() {
    let mut prev = 0.0;
    for k in [0.05, 0.2, 0.4, 0.6, 0.8, 1.0] {
        let e = moon_illuminance(&state(50.0, k), -30.0, 1.0);
        assert!(e > prev, "k {k}: {e} <= {prev}");
        prev = e;
    }
    let mut prev = 0.0;
    for el in [0.0, 3.0, 10.0, 30.0, 60.0, 90.0] {
        let e = moon_illuminance(&state(el, 1.0), -30.0, 1.0);
        assert!(e >= prev, "el {el}");
        prev = e;
    }
    // A half moon gives about 0.3 of the full one (the gamma-compressed Allen function).
    let half = moon_illuminance(&state(50.0, 0.5), -30.0, 1.0);
    let full = moon_illuminance(&state(50.0, 1.0), -30.0, 1.0);
    assert!((half / full - 0.3).abs() < 0.06, "{}", half / full);
}
