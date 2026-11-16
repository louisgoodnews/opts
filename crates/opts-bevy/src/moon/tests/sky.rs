//! The moon in the sky: its position against the sun, the angles, and the lit limb.

use super::*;

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
