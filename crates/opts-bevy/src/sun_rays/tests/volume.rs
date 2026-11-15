//! Tests of the volume that follows the camera and the weather, the tune string and the style.

use super::*;

#[test]
fn the_volume_follows_the_camera_and_the_weather() {
    let mut app = app(Shafts::Medium, false);
    let cam = camera(&mut app, true);
    light(&mut app, LightRole::Sun, true, 20_000.0);
    app.world_mut()
        .insert_resource(SessionEnvironment::default());
    app.update();
    app.update();
    let pos = Vec3::new(40.0, 3.0, -250.0);
    app.world_mut()
        .entity_mut(cam)
        .insert(GlobalTransform::from_translation(pos));
    app.update();
    let (t, v) = app
        .world_mut()
        .query::<(&Transform, &FogVolume)>()
        .single(app.world())
        .map(|(t, v)| (*t, v.clone()))
        .unwrap();
    assert_eq!(t.translation, pos);
    assert_eq!(t.scale, Vec3::splat(SHAFT_VOLUME_SIDE_M));
    assert_eq!(v.density_factor, HAZE_BASE);
    assert_eq!(v.light_intensity, SHAFT_GAIN);
    // Rain under a solid overcast: no direct beam, thick haze.
    let mut env = SessionEnvironment::new(
        crate::CalendarDate::new(2026, 6, 29),
        12.0 * 3600.0,
        "Rainy Afternoon",
    );
    env.refresh_weather(crate::weather::WeatherCatalogue::builtin());
    let wet = env.light_attenuation().shafts;
    app.world_mut().insert_resource(env);
    app.update();
    let v = app
        .world_mut()
        .query::<&FogVolume>()
        .single(app.world())
        .unwrap()
        .clone();
    assert!(v.light_intensity < 0.1 && (v.light_intensity - wet).abs() < 1e-6);
    assert!(v.density_factor >= HAZE_BASE);
}

#[test]
fn the_tune_string_overrides_any_subset() {
    let d = ShaftStyle::default();
    assert_eq!(ShaftStyle::parse_tune(""), d);
    let st = ShaftStyle::parse_tune(
        "haze=0.001, asym=0.6,gain=2,ambient=0.05,absorption=0.1,scatter=0.2,moon_gain=7",
    );
    assert_eq!(
        st,
        ShaftStyle {
            haze: 0.001,
            asym: 0.6,
            gain: 2.0,
            ambient: 0.05,
            absorption: 0.1,
            scatter: 0.2,
            moon_gain: 7.0,
        }
    );
    let part = ShaftStyle::parse_tune("gain=0.25,bogus=3,haze=x,asym=-1,scatter");
    assert_eq!(part, ShaftStyle { gain: 0.25, ..d });
    assert_eq!(ShaftStyle::parse_tune("asym=5").asym, 0.99);
}

#[test]
fn the_style_reaches_the_volume_and_the_camera() {
    let mut app = app(Shafts::Medium, false);
    let st = ShaftStyle::parse_tune(
        "haze=0.004,asym=0.3,gain=1,ambient=0.02,absorption=0.1,scatter=0.2",
    );
    app.insert_resource(st);
    let cam = camera(&mut app, true);
    light(&mut app, LightRole::Sun, true, 20_000.0);
    app.update();
    app.update();
    let v = app
        .world_mut()
        .query::<&FogVolume>()
        .single(app.world())
        .unwrap()
        .clone();
    assert_eq!(
        (
            v.density_factor,
            v.scattering_asymmetry,
            v.light_intensity,
            v.absorption,
            v.scattering
        ),
        (0.004, 0.3, 1.0, 0.1, 0.2)
    );
    assert_eq!(
        app.world()
            .get::<VolumetricFog>(cam)
            .unwrap()
            .ambient_intensity,
        0.02
    );
}
