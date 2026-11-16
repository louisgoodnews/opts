//! Weather changes through the catalogue, the plugin, and the cloud attenuation.

use super::*;

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
