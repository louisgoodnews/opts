//! Unit tests of `sun_rays`.

use super::*;
use bevy::camera::Hdr;
use bevy::light::{FogVolume, VolumetricFog, VolumetricLight};
use crate::light_quality::{LightQuality, Shafts};

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
    const { assert!(HAZE_MAX <= 0.01) };
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

mod live;
mod volume;
