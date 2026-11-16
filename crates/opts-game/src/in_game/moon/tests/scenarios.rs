//! The moon scenarios: a full moon night, a new moon night, and the moon light's direction, run in
//! the harness app with the environment's clock.

use opts_bevy::CalendarDate;

use super::super::*;
use super::harness::{app, set_time, state};

#[test]
fn one_light_carries_the_shadows_through_a_full_moon_day() {
    let mut app = app();
    // 2026-10-26: full moon (04:12 UTC).
    let date = CalendarDate::new(2026, 10, 26);
    let mut carriers = Vec::new();
    let mut moon_lit_hours = 0;
    for step in 0..96 {
        let hour = step as f32 * 0.25;
        set_time(&mut app, date, hour);
        let (shadows, lux, hidden) = state(&mut app);
        let env = app.world().resource::<SessionEnvironment>();
        let (sun_up, moon_up) = (env.sun_alt_deg() > 0.0, env.moon().elevation_deg > 0.0);
        assert!(shadows <= 1, "{hour}: {shadows} shadow lights");
        let carrier = app.world().resource::<ShadowCarrier>().0;
        if sun_up {
            if env.sun_alt_deg() >= 6.0 {
                assert_eq!(lux, 0.0, "{hour}: no moonlight by day");
                assert!(hidden, "{hour}");
            }
            assert_eq!(carrier, Some(LightRole::Sun), "{hour}");
            assert_eq!(shadows, 1);
        } else if moon_up && lux >= MIN_LIGHT_LX {
            moon_lit_hours += 1;
            assert_eq!(carrier, Some(LightRole::Moon), "{hour}: lux {lux}");
            assert_eq!(shadows, 1);
            assert!(!hidden);
        } else {
            assert_eq!(carrier, None, "{hour}");
            assert_eq!(shadows, 0, "{hour}");
        }
        if carriers.last() != Some(&carrier) {
            carriers.push(carrier);
        }
    }
    assert!(moon_lit_hours >= 4 * 6, "the full moon lights the night");
    // Sun, (nothing or moon) ..., sun: no flicker between.
    assert!(carriers.len() <= 5, "{carriers:?}");
}

#[test]
fn a_new_moon_night_has_no_moonlight_and_no_shadows() {
    let mut app = app();
    // 2026-11-09: new moon (about 08:00 UTC).
    let date = CalendarDate::new(2026, 11, 9);
    for hour in [0.0, 2.0, 22.0, 23.5] {
        set_time(&mut app, date, hour);
        let (shadows, lux, hidden) = state(&mut app);
        assert_eq!(lux, 0.0, "{hour}");
        assert!(hidden);
        assert_eq!(shadows, 0, "{hour}");
    }
    set_time(&mut app, date, 12.0);
    assert_eq!(state(&mut app).0, 1, "the sun carries by day");
}

#[test]
fn the_moon_light_points_from_the_model_direction() {
    let mut app = app();
    set_time(&mut app, CalendarDate::new(2026, 10, 26), 0.0);
    let env = app.world().resource::<SessionEnvironment>().clone();
    let moon = env.moon();
    assert!(moon.elevation_deg > 0.0, "{moon:?}");
    let t = app
        .world_mut()
        .query_filtered::<&Transform, With<MoonLight>>()
        .single(app.world())
        .unwrap();
    let towards_moon = -*t.forward();
    let want = direction(moon.elevation_deg, moon.azimuth_deg);
    assert!(towards_moon.dot(want) > 0.999);
}
