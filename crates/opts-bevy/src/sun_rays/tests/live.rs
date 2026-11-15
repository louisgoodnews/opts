//! Tests of the volumetric components that follow the quality and the carrier light.

use super::*;

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
