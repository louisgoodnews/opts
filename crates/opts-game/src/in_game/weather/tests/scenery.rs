//! Weather values read by scenery and vehicle scripts, and the ground fog by camera height.

use super::*;

#[test]
fn scenery_and_vehicle_scripts_get_the_weather_values() {
    use opts_bevy::SceneryClock;
    let mut app = app();
    app.init_resource::<SceneryClock>().add_systems(
        Update,
        crate::in_game::omsi_world::sync_scenery_clock.after(EnvironmentSet),
    );
    set_weather(&mut app, "Rainy Afternoon");
    let clock = app.world().resource::<SceneryClock>();
    assert_eq!(clock.weather.precip_type, 1.0);
    assert!(
        clock.weather.precip_rate > 0.4,
        "{}",
        clock.weather.precip_rate
    );
    assert!(clock.weather.abs_hum > 5.0);
    assert_eq!(clock.temperature_c, 15.0);
    set_weather(&mut app, "Winter Snowfall");
    let clock = app.world().resource::<SceneryClock>();
    assert_eq!(
        (clock.weather.precip_type, clock.temperature_c),
        (2.0, -3.0)
    );
    set_weather(&mut app, "Clear Summer Day");
    assert_eq!(
        app.world().resource::<SceneryClock>().weather.precip_type,
        0.0
    );
    // A vehicle's scripts read them as system and local variables.
    let (mut engine, _) =
        opts_sim::script::ScriptEngine::from_sources(&[("weather.osc", "{frame}\n")]);
    set_weather(&mut app, "Rainy Afternoon");
    let clock = app.world().resource::<SceneryClock>().clone();
    opts_bevy::vehicle_script::feed_weather(&mut engine, clock.temperature_c, &clock.weather);
    assert_eq!(engine.get("preciptype"), 1.0);
    assert!((engine.get("preciprate") - 173.0 / 255.0).abs() < 0.3);
}

#[test]
fn ground_fog_follows_the_camera_height_and_other_presets_do_not() {
    use opts_bevy::weather::GroundFogView;
    let mut app = app();
    set_weather(&mut app, "Autumn Morning Fog");
    // 300 m dusk-like fog: no layer, the same density at any height.
    place_camera(&mut app, 2.0, 0.0);
    let flat = fog_density_of(&mut app);
    place_camera(&mut app, 70.0, 0.0);
    assert_eq!(fog_density_of(&mut app), flat);
    assert_eq!(app.world().resource::<GroundFogView>().layer_m, None);

    // An installed ground fog (75 m, like `Bodennebel`) is a layer.
    let mut w = opts_core::formats::weather::WeatherFile::default();
    w.fog.visibility_m = 75.0;
    w.fog.density = 1.0;
    let folder = opts_core::formats::weather::WeatherFolder {
        presets: vec![opts_core::formats::weather::PresetFile {
            id: "Bodennebel".into(),
            weather: w,
            texts: Default::default(),
        }],
        clouds: Default::default(),
    };
    app.insert_resource(WeatherCatalogue::from_folder("/nonexistent".into(), folder));
    set_weather(&mut app, "Bodennebel");
    place_camera(&mut app, 2.0, 0.0);
    let low = fog_density_of(&mut app);
    place_camera(&mut app, 20.0, 0.0);
    let mid = fog_density_of(&mut app);
    place_camera(&mut app, 70.0, 0.0);
    let high = fog_density_of(&mut app);
    assert!(low > mid && mid > high, "{low} {mid} {high}");
    let view = *app.world().resource::<GroundFogView>();
    assert_eq!(view.layer_m, Some(30.0));
    assert!(view.sky_clear > 0.99 && view.effective_visibility_m > 170.0);
    // Looking up from above the layer clears the fog further; at the ground nothing changes.
    place_camera(&mut app, 70.0, 20.0);
    assert!(fog_density_of(&mut app) < high);
    place_camera(&mut app, 2.0, 20.0);
    assert_eq!(fog_density_of(&mut app), low);

    // The clear preset has no fog at all, at any height.
    set_weather(&mut app, "Clear Summer Day");
    place_camera(&mut app, 70.0, 0.0);
    assert!(!camera_has_fog(&mut app));
}
