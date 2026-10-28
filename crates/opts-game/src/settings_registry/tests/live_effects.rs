//! Tests for live settings synchronization: gameplay baselines, weather effects and HDR.

use super::*;

#[test]
fn gameplay_defaults_equal_the_constants_of_the_session_systems() {
    use crate::in_game::omsi_world::{DEFAULT_PEDESTRIAN_BASELINE, DEFAULT_TRAFFIC_BASELINE};
    let r = registry();
    assert_eq!(
        r.get("gameplay.traffic").map(|d| d.default),
        Some(DEFAULT_TRAFFIC_BASELINE.to_string().as_str())
    );
    assert_eq!(
        r.get("gameplay.pedestrians").map(|d| d.default),
        Some(DEFAULT_PEDESTRIAN_BASELINE.to_string().as_str())
    );
}

#[test]
fn the_weather_settings_default_on_and_switch_each_effect() {
    use opts_bevy::weather::PrecipitationQuality;
    let r = SettingsRegistry::with_builtin();
    for key in [
        "graphics.weather_fog",
        "graphics.weather_clouds",
        "graphics.weather_wet_ground",
    ] {
        assert_eq!(r.get(key).map(|d| d.default), Some("true"), "{key}");
    }
    assert_eq!(
        r.get("graphics.weather_precipitation").map(|d| d.default),
        Some("medium")
    );
    let mut settings = GameSettings::default();
    assert_eq!(
        live::weather_effects_of(&settings),
        opts_bevy::WeatherEffects::default()
    );
    for (key, value) in [
        ("graphics.weather_fog", "false"),
        ("graphics.weather_clouds", "false"),
        ("graphics.weather_wet_ground", "false"),
        ("graphics.weather_precipitation", "high"),
    ] {
        settings.set_raw(key, value);
    }
    let fx = live::weather_effects_of(&settings);
    assert!(!fx.fog && !fx.clouds && !fx.wet_ground);
    assert_eq!(fx.precipitation, PrecipitationQuality::High);
    // An unknown stored quality falls back to the default instead of switching rain off.
    settings.set_raw("graphics.weather_precipitation", "ultra");
    assert_eq!(
        live::weather_effects_of(&settings).precipitation,
        PrecipitationQuality::Medium
    );

    // The system keeps the resource equal to the settings.
    let mut app = App::new();
    app.insert_resource(settings)
        .init_resource::<opts_bevy::WeatherEffects>()
        .add_systems(Update, live::sync_weather_effects);
    app.update();
    assert!(!app.world().resource::<opts_bevy::WeatherEffects>().fog);
    app.world_mut()
        .resource_mut::<GameSettings>()
        .set_raw("graphics.weather_fog", "true");
    app.update();
    assert!(app.world().resource::<opts_bevy::WeatherEffects>().fog);
}

#[test]
fn hdr_is_on_by_default_and_follows_the_setting_on_the_main_camera_only() {
    use bevy::camera::Hdr;
    let r = SettingsRegistry::with_builtin();
    assert_eq!(r.get("graphics.hdr").map(|d| d.default), Some("true"));
    let mut app = App::new();
    app.insert_resource(GameSettings::default())
        .add_systems(Update, live::sync_hdr);
    let main = app
        .world_mut()
        .spawn((Camera3d::default(), opts_bevy::MainCamera))
        .id();
    let mirror = app.world_mut().spawn(Camera3d::default()).id();
    app.update();
    assert!(app.world().get::<Hdr>(main).is_some(), "on by default");
    assert!(
        app.world().get::<Hdr>(mirror).is_none(),
        "not on other cameras"
    );
    app.world_mut()
        .resource_mut::<GameSettings>()
        .set_raw("graphics.hdr", "false");
    app.update();
    assert!(app.world().get::<Hdr>(main).is_none(), "off, live");
    app.world_mut()
        .resource_mut::<GameSettings>()
        .set_raw("graphics.hdr", "true");
    app.update();
    assert!(app.world().get::<Hdr>(main).is_some(), "on again, live");
}
