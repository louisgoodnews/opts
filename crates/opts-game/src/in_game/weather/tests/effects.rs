//! Live weather effects: fog, precipitation, wetness and the settings that switch them off.

use super::*;

#[test]
fn a_weather_change_updates_fog_and_precipitation_live() {
    let mut app = app();
    assert!(!camera_has_fog(&mut app), "clear air has no fog");
    assert!(!precip_visible(&mut app));

    set_weather(&mut app, "Autumn Morning Fog");
    assert!(camera_has_fog(&mut app));

    set_weather(&mut app, "Rainy Afternoon");
    assert!(camera_has_fog(&mut app), "6 km visibility is hazy");
    let state = app.world().resource::<precip::PrecipState>();
    assert_eq!(state.kind(), Some(PrecipKind::Rain));
    assert!(state.drawn() > 0 && state.drawn() <= state.allocated());
    assert!(precip_visible(&mut app));
    let rain_quads = app.world().resource::<precip::PrecipState>().drawn();

    set_weather(&mut app, "Winter Snowfall");
    let state = app.world().resource::<precip::PrecipState>();
    assert_eq!(state.kind(), Some(PrecipKind::Snow));
    assert!(state.drawn() > 0);
    assert_ne!(state.drawn(), 0, "{rain_quads}");

    set_weather(&mut app, "Clear Summer Day");
    assert!(!camera_has_fog(&mut app));
    assert!(!precip_visible(&mut app));
    assert_eq!(app.world().resource::<precip::PrecipState>().kind(), None);
}

#[test]
fn the_settings_switch_each_effect_off_live() {
    let mut app = app();
    set_weather(&mut app, "Rainy Afternoon");
    assert!(camera_has_fog(&mut app) && precip_visible(&mut app));

    app.world_mut().resource_mut::<WeatherEffects>().fog = false;
    app.update();
    assert!(!camera_has_fog(&mut app));

    app.world_mut()
        .resource_mut::<WeatherEffects>()
        .precipitation = PrecipitationQuality::Off;
    app.update();
    assert!(!precip_visible(&mut app));

    // Back on, with a smaller budget: fewer quads than the medium one.
    app.world_mut()
        .resource_mut::<WeatherEffects>()
        .precipitation = PrecipitationQuality::Medium;
    app.update();
    let medium = app.world().resource::<precip::PrecipState>().drawn();
    app.world_mut()
        .resource_mut::<WeatherEffects>()
        .precipitation = PrecipitationQuality::Low;
    app.update();
    let low = app.world().resource::<precip::PrecipState>().drawn();
    assert!(low > 0 && low < medium, "{low} < {medium}");
    assert!(low <= PrecipitationQuality::Low.budget() as usize);
}

#[test]
fn the_weather_drives_the_wetness_resource_and_the_setting_dries_it() {
    use opts_bevy::reflections::Wetness;
    let mut app = app();
    assert_eq!(app.world().resource::<Wetness>().0, 0.0);
    set_weather(&mut app, "Rainy Afternoon");
    assert!(app.world().resource::<Wetness>().0 > 0.9);
    app.world_mut().resource_mut::<WeatherEffects>().wet_ground = false;
    app.update();
    assert_eq!(app.world().resource::<Wetness>().0, 0.0);
}

#[test]
fn the_session_environment_carries_the_conditions_the_effects_read() {
    let mut app = app();
    set_weather(&mut app, "Rainy Afternoon");
    let env = app.world().resource::<SessionEnvironment>();
    assert_eq!(env.conditions().precip, PrecipKind::Rain);
    assert!(env.conditions().fog_visibility_m < 10_000.0);
}

