//! Unit tests for `super` (`moon.rs`).

use super::*;
use opts_bevy::{CalendarDate, EnvironmentChange, EnvironmentPlugin};

#[test]
fn the_carrier_keeps_hysteresis_and_prefers_the_sun_on_ties() {
    assert_eq!(choose_carrier(None, 100.0, 100.0), Some(LightRole::Sun));
    assert_eq!(choose_carrier(None, 0.0, 0.0), None);
    assert_eq!(choose_carrier(None, 0.5, 0.2), None);
    assert_eq!(choose_carrier(None, 0.0, 5.0), Some(LightRole::Moon));
    // 14 % better is not enough, 16 % is.
    assert_eq!(
        choose_carrier(Some(LightRole::Sun), 100.0, 114.0),
        Some(LightRole::Sun)
    );
    assert_eq!(
        choose_carrier(Some(LightRole::Sun), 100.0, 116.0),
        Some(LightRole::Moon)
    );
    // A carrier that fell dark hands over to any light.
    assert_eq!(
        choose_carrier(Some(LightRole::Sun), 0.0, 5.0),
        Some(LightRole::Moon)
    );
    assert_eq!(choose_carrier(Some(LightRole::Sun), 0.0, 0.0), None);
}

#[test]
fn the_sun_fading_through_the_hand_over_flips_once() {
    // The sun's illuminance falls from 800 to 0 against a steady 900 lx moon: one switch.
    let mut cur = Some(LightRole::Sun);
    let mut switches = 0;
    for i in 0..=100 {
        let sun = 1500.0 * (1.0 - i as f32 / 100.0);
        let moon = 900.0 + if i % 2 == 0 { 40.0 } else { -40.0 };
        let next = choose_carrier(cur, sun, moon);
        if next != cur {
            switches += 1;
        }
        cur = next;
    }
    assert_eq!(switches, 1);
    assert_eq!(cur, Some(LightRole::Moon));
}

#[test]
fn the_disc_is_faint_by_day_and_dimmed_by_cloud() {
    assert_eq!(disc_brightness(-30.0, 1.0), 1.0);
    assert!((disc_brightness(40.0, 1.0) - MOON_DISC_DAY_BRIGHTNESS).abs() < 1e-6);
    assert!(disc_brightness(-30.0, 0.35) < 0.4);
}

fn fake_sun(env: Res<SessionEnvironment>, mut sun: Query<&mut DirectionalLight, With<SunLight>>) {
    for mut l in &mut sun {
        l.illuminance = if env.sun_alt_deg() > 0.0 {
            20_000.0
        } else {
            0.0
        };
    }
}

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(EnvironmentPlugin)
        .init_resource::<ShadowCarrier>()
        .add_systems(
            Update,
            (
                spawn_moon_light,
                fake_sun,
                update_moon,
                shadow_carrier_system,
            )
                .chain()
                .after(opts_bevy::EnvironmentSet),
        );
    app.world_mut().spawn((
        DirectionalLight {
            shadow_maps_enabled: true,
            ..default()
        },
        SunLight,
        LightRole::Sun,
    ));
    app
}

fn set_time(app: &mut App, date: CalendarDate, hour: f32) {
    app.world_mut().write_message(EnvironmentChange::reset(
        date,
        f64::from(hour) * 3600.0,
        "Clear Summer Day",
    ));
    app.update();
}

/// (shadow-casting lights, moon illuminance, moon hidden)
fn state(app: &mut App) -> (usize, f32, bool) {
    let w = app.world_mut();
    let shadows = w
        .query::<&DirectionalLight>()
        .iter(w)
        .filter(|l| l.shadow_maps_enabled)
        .count();
    let (l, v) = w
        .query_filtered::<(&DirectionalLight, &Visibility), With<MoonLight>>()
        .single(w)
        .expect("moon light");
    (shadows, l.illuminance, *v == Visibility::Hidden)
}

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
