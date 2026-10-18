use super::*;
use crate::light_quality::LightQuality;

fn app(shafts: Shafts, night: bool) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins).add_plugins(SunRaysPlugin);
    *app.world_mut().resource_mut::<LightQuality>() = LightQuality {
        shafts,
        night_shafts: night,
        ..LightQuality::default()
    };
    app
}

fn light(app: &mut App, role: LightRole, shadows: bool, lux: f32) -> Entity {
    app.world_mut()
        .spawn((
            role,
            DirectionalLight {
                shadow_maps_enabled: shadows,
                illuminance: lux,
                ..default()
            },
        ))
        .id()
}

fn camera(app: &mut App, hdr: bool) -> Entity {
    let mut e = app
        .world_mut()
        .spawn((MainCamera, GlobalTransform::default()));
    if hdr {
        e.insert(Hdr);
    }
    e.id()
}

fn count<T: Component>(app: &mut App) -> usize {
    app.world_mut().query::<&T>().iter(app.world()).count()
}

fn set_quality(app: &mut App, f: impl FnOnce(&mut LightQuality)) {
    f(&mut app.world_mut().resource_mut::<LightQuality>());
}

#[test]
fn haze_is_the_baseline_in_clear_air_and_follows_fog() {
    let st = ShaftStyle::default();
    assert_eq!(haze_density(15_000.0, &st), HAZE_BASE);
    assert_eq!(haze_density(10_000.0, &st), HAZE_BASE);
    assert!(haze_density(2_000.0, &st) > HAZE_BASE);
    assert!((haze_density(800.0, &st) - 3.912 / 800.0 * FOG_TO_SHAFT).abs() < 1e-6);
    // Thick fog is capped: the haze must not become a second, sun-coloured fog.
    assert_eq!(haze_density(2.0, &st), HAZE_MAX);
    assert_eq!(haze_density(75.0, &st), HAZE_MAX);
    assert!(HAZE_MAX <= 0.01);
    // Thicker fog never gives a thinner haze.
    let mut prev = 0.0;
    for v in [10_000.0, 2_000.0, 500.0, 100.0, 20.0] {
        let d = haze_density(v, &st);
        assert!(d >= prev);
        prev = d;
    }
}

#[test]
fn intensity_follows_role_weather_and_hdr() {
    let st = ShaftStyle::default();
    assert_eq!(shaft_intensity(LightRole::Sun, 1.0, true, &st), st.gain);
    assert_eq!(shaft_intensity(LightRole::Sun, 0.0, true, &st), 0.0);
    assert!(shaft_intensity(LightRole::Moon, 1.0, true, &st) > st.gain);
    assert!(shaft_intensity(LightRole::Sun, 1.0, false, &st) < st.gain);
}

#[test]
fn the_carrier_needs_shadows_light_and_an_allowed_role() {
    let l = [
        (LightRole::Sun, false, 100.0),
        (LightRole::Moon, true, 50.0),
    ];
    assert_eq!(pick_carrier(Shafts::Medium, true, l), Some(1));
    assert_eq!(pick_carrier(Shafts::Medium, false, l), None);
    assert_eq!(pick_carrier(Shafts::Off, true, l), None);
    let l = [(LightRole::Sun, true, 0.0), (LightRole::Moon, false, 5.0)];
    assert_eq!(pick_carrier(Shafts::High, true, l), None);
}

#[test]
fn every_quality_gives_the_expected_components() {
    for (shafts, steps) in [
        (Shafts::Low, 8),
        (Shafts::Medium, 16),
        (Shafts::High, 32),
        (Shafts::Ultra, 64),
    ] {
        let mut app = app(shafts, true);
        let cam = camera(&mut app, true);
        let sun = light(&mut app, LightRole::Sun, true, 20_000.0);
        let moon = light(&mut app, LightRole::Moon, false, 500.0);
        app.update();
        app.update();
        let fog = app.world().get::<VolumetricFog>(cam).expect("fog");
        assert_eq!(fog.step_count, steps);
        assert_eq!(fog.ambient_intensity, 0.0);
        assert!(app.world().get::<VolumetricLight>(sun).is_some());
        assert!(app.world().get::<VolumetricLight>(moon).is_none());
        assert_eq!(count::<FogVolume>(&mut app), 1);
        assert_eq!(count::<VolumetricLight>(&mut app), 1);
    }
    let mut app = app(Shafts::Off, true);
    let cam = camera(&mut app, true);
    light(&mut app, LightRole::Sun, true, 20_000.0);
    app.update();
    app.update();
    assert!(app.world().get::<VolumetricFog>(cam).is_none());
    assert_eq!(count::<VolumetricLight>(&mut app), 0);
    assert_eq!(count::<FogVolume>(&mut app), 0);
}

#[test]
fn quality_changes_are_followed_live_and_off_removes_everything() {
    let mut app = app(Shafts::Medium, true);
    let cam = camera(&mut app, true);
    light(&mut app, LightRole::Sun, true, 20_000.0);
    app.update();
    app.update();
    assert_eq!(count::<FogVolume>(&mut app), 1);
    set_quality(&mut app, |q| q.shafts = Shafts::High);
    app.update();
    assert_eq!(
        app.world().get::<VolumetricFog>(cam).unwrap().step_count,
        32
    );
    assert_eq!(count::<FogVolume>(&mut app), 1, "no second volume");
    set_quality(&mut app, |q| q.shafts = Shafts::Off);
    app.update();
    app.update();
    assert!(app.world().get::<VolumetricFog>(cam).is_none());
    assert_eq!(count::<VolumetricLight>(&mut app), 0);
    assert_eq!(count::<FogVolume>(&mut app), 0);
    set_quality(&mut app, |q| q.shafts = Shafts::Low);
    app.update();
    app.update();
    assert_eq!(app.world().get::<VolumetricFog>(cam).unwrap().step_count, 8);
    assert_eq!(count::<FogVolume>(&mut app), 1);
}

#[test]
fn the_volumetric_light_follows_the_shadow_carrier() {
    let mut app = app(Shafts::Medium, true);
    camera(&mut app, true);
    let sun = light(&mut app, LightRole::Sun, true, 20_000.0);
    let moon = light(&mut app, LightRole::Moon, false, 0.0);
    app.update();
    app.update();
    assert!(app.world().get::<VolumetricLight>(sun).is_some());
    // The game's carrier system hands the shadows over.
    let swap = |app: &mut App, sun_shadows: bool| {
        app.world_mut()
            .get_mut::<DirectionalLight>(sun)
            .unwrap()
            .shadow_maps_enabled = sun_shadows;
        let mut m = app.world_mut().get_mut::<DirectionalLight>(moon).unwrap();
        m.shadow_maps_enabled = !sun_shadows;
        m.illuminance = 400.0;
    };
    swap(&mut app, false);
    app.update();
    app.update();
    assert!(app.world().get::<VolumetricLight>(sun).is_none());
    assert!(app.world().get::<VolumetricLight>(moon).is_some());
    assert_eq!(count::<VolumetricLight>(&mut app), 1);
    // Night shafts off: the moon carries the shadows but gets no rays, and the rest goes.
    set_quality(&mut app, |q| q.night_shafts = false);
    app.update();
    app.update();
    assert_eq!(count::<VolumetricLight>(&mut app), 0);
    assert_eq!(count::<FogVolume>(&mut app), 0);
    swap(&mut app, true);
    app.update();
    app.update();
    assert!(app.world().get::<VolumetricLight>(sun).is_some());
}

#[test]
fn no_carrier_or_no_camera_means_nothing_runs() {
    let mut app = app(Shafts::Medium, true);
    let cam = camera(&mut app, false);
    let sun = light(&mut app, LightRole::Sun, false, 20_000.0);
    app.update();
    app.update();
    assert!(app.world().get::<VolumetricFog>(cam).is_none());
    assert_eq!(count::<FogVolume>(&mut app), 0);
    app.world_mut()
        .get_mut::<DirectionalLight>(sun)
        .unwrap()
        .shadow_maps_enabled = true;
    app.update();
    app.update();
    assert!(
        app.world().get::<VolumetricFog>(cam).is_some(),
        "works without Hdr"
    );
    app.world_mut().despawn(cam);
    app.update();
    app.update();
    assert_eq!(count::<FogVolume>(&mut app), 0);
    assert_eq!(count::<VolumetricLight>(&mut app), 0);
}

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
