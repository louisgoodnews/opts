//! The moonlight: zero below the horizon and at new moon, growing with the phase and the height.

use super::*;

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
