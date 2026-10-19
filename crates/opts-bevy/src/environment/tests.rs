//! Unit tests of `environment.rs` (moved out of the file to keep it under the 500-line limit, refactoring cycle 1).

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
fn changes_bump_the_revision_and_follow_the_map_date() {
    let mut env = SessionEnvironment::new(CalendarDate::new(1990, 6, 1), 43_200.0, "Clear");
    let r0 = env.revision();
    assert_eq!(
        env.apply(&EnvironmentChange {
            rate: Some(10.0),
            ..default()
        }),
        None
    );
    assert_eq!(env.revision(), r0, "a rate is not a jump");
    assert_eq!(env.rate(), 10.0);
    env.apply(&EnvironmentChange {
        seconds_of_day: Some(90_000.0),
        weather: Some("Winter Snowfall".into()),
        ..default()
    });
    assert_eq!(env.revision(), r0 + 1);
    assert!((env.seconds_of_day() - 3600.0).abs() < 1e-6);
    assert_eq!(env.weather_kind(), WeatherKind::Snow);
    assert_eq!(env.temperature_c(), -3.0);
    assert!(env.attenuation() < 1.0);
    assert!(env.night());
    // A previewed date keeps the map date; Follow moves it and announces it once.
    env.apply(&EnvironmentChange {
        date: Some(CalendarDate::new(1991, 1, 1)),
        ..default()
    });
    assert_eq!(env.map_date(), CalendarDate::new(1990, 6, 1));
    let ann = env.apply(&EnvironmentChange {
        date: Some(CalendarDate::new(1992, 2, 2)),
        map_date: MapDatePolicy::Follow,
        ..default()
    });
    assert_eq!(
        ann,
        Some(MapDateChanged {
            old: CalendarDate::new(1990, 6, 1),
            new: CalendarDate::new(1992, 2, 2)
        })
    );
    assert_eq!(
        env.apply(&EnvironmentChange {
            date: Some(CalendarDate::new(1992, 2, 2)),
            map_date: MapDatePolicy::Follow,
            ..default()
        }),
        None
    );
    let reset = EnvironmentChange::reset(CalendarDate::new(2000, 1, 1), 0.0, "x");
    assert_eq!(env.apply(&reset), None);
    assert_eq!(env.map_date(), CalendarDate::new(2000, 1, 1));
    assert_eq!(env.rate(), 1.0);
}

#[test]
fn sun_and_night_follow_the_clock() {
    let noon = SessionEnvironment::new(CalendarDate::new(2026, 6, 29), 12.0 * 3600.0, "");
    assert!(noon.sun_alt_deg() > 50.0 && !noon.night());
    assert!((noon.sun_az_deg() - 180.0).abs() < 1.0);
    let late = SessionEnvironment::new(CalendarDate::new(2026, 6, 29), 23.0 * 3600.0, "");
    assert!(late.night());
}

#[test]
fn traffic_density_reads_the_wrapped_hour_and_the_weekend() {
    use opts_core::formats::map::lists::{DensityProfile, GroupDensity};
    let group = GroupDensity {
        name: "cars".into(),
        factor: 1.0,
        profiles: vec![
            DensityProfile {
                days: day::WEEKDAY,
                points: vec![(0.0, 0.1), (8.0, 1.0), (24.0, 0.1)],
            },
            DensityProfile {
                days: day::SATURDAY | day::SUNDAY,
                points: vec![(0.0, 0.05), (24.0, 0.05)],
            },
        ],
    };
    // Friday 23:00 + 9 h = Saturday 08:00: the hour wraps (8, not 32) and the weekend curve
    // applies.
    let mut env = SessionEnvironment::new(CalendarDate::new(2026, 10, 16), 23.0 * 3600.0, "");
    assert_eq!(env.day_bit(), day::WEEKDAY);
    env.advance(9.0 * 3600.0);
    assert!((env.hour() - 8.0).abs() < 1e-4);
    assert_eq!(env.day_bit(), day::SATURDAY);
    assert!((group.density(env.day_bit(), env.hour()) - 0.05).abs() < 1e-6);
    let monday = SessionEnvironment::new(CalendarDate::new(2026, 10, 19), 8.0 * 3600.0, "");
    assert!((group.density(monday.day_bit(), monday.hour()) - 1.0).abs() < 1e-6);
}

#[test]
fn weather_kinds_from_preset_words() {
    assert_eq!(
        WeatherKind::of_preset("Clear Summer Day"),
        WeatherKind::Clear
    );
    assert_eq!(
        WeatherKind::of_preset("Overcast Autumn"),
        WeatherKind::Overcast
    );
    assert_eq!(WeatherKind::of_preset("Rainy Afternoon"), WeatherKind::Rain);
    assert_eq!(
        WeatherKind::of_preset("Autumn Morning Fog"),
        WeatherKind::Fog
    );
    assert_eq!(WeatherKind::of_preset("Winter Snowfall"), WeatherKind::Snow);
}

#[test]
fn the_systems_tick_with_virtual_time_and_apply_changes() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(EnvironmentPlugin);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_millis(500),
    ));
    app.world_mut().write_message(EnvironmentChange {
        rate: Some(4.0),
        ..EnvironmentChange::reset(CalendarDate::new(1994, 12, 31), 100.0, "Clear")
    });
    for _ in 0..3 {
        app.update();
    }
    let env = app.world().resource::<SessionEnvironment>();
    assert_eq!(env.rate(), 4.0);
    // The first frames have no delta; the clock moved by 0.5 s × 4 per frame after that.
    assert!(env.seconds_of_day() > 100.0, "{}", env.seconds_of_day());
    app.world_mut().write_message(EnvironmentChange {
        date: Some(CalendarDate::new(1995, 1, 2)),
        map_date: MapDatePolicy::Follow,
        ..default()
    });
    app.update();
    let msgs = app.world().resource::<Messages<MapDateChanged>>();
    let mut cursor = msgs.get_cursor();
    let got: Vec<_> = cursor.read(msgs).copied().collect();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].new, CalendarDate::new(1995, 1, 2));
}

#[test]
fn a_weather_change_resolves_through_the_catalogue_and_is_a_jump() {
    use opts_core::formats::weather::{PresetFile, WeatherFile, WeatherFolder};
    let storm = WeatherFile {
        fog: opts_core::formats::weather::Fog {
            visibility_m: 700.0,
            density: 0.5,
        },
        temp: opts_core::formats::weather::Temperature {
            celsius: 9.0,
            dew_point_c: 9.0,
        },
        ..WeatherFile::default()
    };
    let catalogue = WeatherCatalogue::from_folder(
        "/omsi".into(),
        WeatherFolder {
            presets: vec![PresetFile {
                id: "Sturm".into(),
                weather: storm,
                texts: Default::default(),
            }],
            ..default()
        },
    );
    let mut env = SessionEnvironment::new(CalendarDate::new(2026, 6, 29), 43_200.0, "Sturm");
    // Without the install the id is unknown: clear weather.
    assert_eq!(env.conditions().fog_visibility_m, 50_000.0);
    let r0 = env.revision();
    env.refresh_weather(&catalogue);
    assert_eq!(env.conditions().fog_visibility_m, 700.0);
    assert_eq!(env.temperature_c(), 9.0);
    assert_eq!(env.revision(), r0 + 1, "a different outcome is a jump");
    env.refresh_weather(&catalogue);
    assert_eq!(env.revision(), r0 + 1, "the same outcome is not");
    env.apply_with(
        &EnvironmentChange {
            weather: Some("Rainy Afternoon".into()),
            ..default()
        },
        &catalogue,
    );
    assert_eq!(env.conditions().kind(), WeatherKind::Rain);
    assert_eq!(
        env.weather(),
        "Rainy Afternoon",
        "the stored id is kept as given"
    );
}

#[test]
fn the_plugin_resolves_weather_changes_and_catalogue_changes() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(EnvironmentPlugin);
    app.world_mut().write_message(EnvironmentChange::reset(
        CalendarDate::new(2026, 6, 29),
        43_200.0,
        "Rainy Afternoon",
    ));
    app.update();
    let env = app.world().resource::<SessionEnvironment>();
    assert_eq!(
        env.conditions().precip,
        opts_core::formats::weather::PrecipKind::Rain
    );
    assert!(env.attenuation() < 0.6);
}

fn sky(cover: f32) -> WeatherSky {
    WeatherSky {
        cloud_cover: cover,
        ..WeatherSky::default()
    }
}

#[test]
fn attenuation_is_clear_at_one_and_the_old_values_under_full_cover() {
    let clear = light_attenuation(&sky(0.0));
    assert_eq!(
        (clear.direct, clear.ambient, clear.moon, clear.shafts),
        (1.0, 1.0, 1.0, 1.0)
    );
    let over = light_attenuation(&sky(1.0));
    assert!((over.direct - 0.55).abs() < 1e-6);
    assert!((over.ambient - 0.8).abs() < 1e-6);
    assert!((over.moon - 0.35).abs() < 1e-6);
    assert_eq!(over.shafts, 0.0, "a solid overcast has no beam");
}

#[test]
fn attenuation_is_monotone_in_cloud_cover() {
    let mut prev = light_attenuation(&sky(0.0));
    for i in 1..=20 {
        let a = light_attenuation(&sky(i as f32 / 20.0));
        assert!(a.direct <= prev.direct && a.ambient <= prev.ambient);
        assert!(a.moon <= prev.moon && a.shafts <= prev.shafts);
        assert!(a.moon < a.direct, "cloud blocks the weaker light more");
        prev = a;
    }
}

#[test]
fn the_environment_wrappers_agree_with_the_function() {
    let env = SessionEnvironment::new(CalendarDate::new(2026, 6, 29), 43_200.0, "Rainy Afternoon");
    let a = env.light_attenuation();
    assert_eq!(env.attenuation(), a.direct);
    assert_eq!(env.ambient_attenuation(), a.ambient);
    assert_eq!(
        a,
        light_attenuation(&WeatherSky::from_conditions(env.conditions()))
    );
    assert!(a.shafts < 0.2 && a.moon < 0.6);
}

#[test]
fn the_moon_follows_the_clock() {
    let mut env = SessionEnvironment::new(CalendarDate::new(2026, 10, 17), 18.0 * 3600.0, "");
    let before = *env.moon();
    env.advance(4.0 * 3600.0);
    assert_ne!(before.elevation_deg, env.moon().elevation_deg);
    assert!((before.illuminated - env.moon().illuminated).abs() < 0.02);
}
