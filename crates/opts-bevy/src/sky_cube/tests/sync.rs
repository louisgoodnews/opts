//! The sync system: the main camera gets one shared cube, glass reflections toggle, and the
//! intensity follows the day without a rebuild.

use super::*;

fn app_with_camera() -> (App, Entity) {
    let mut app = App::new();
    app.init_resource::<Time<Real>>()
        .init_resource::<Assets<Image>>()
        .insert_resource(LightQuality::default())
        .init_resource::<SkyCubeState>()
        .insert_resource(input())
        .add_systems(Update, sync_sky_cube);
    let cam = app
        .world_mut()
        .spawn((Camera3d::default(), MainCamera))
        .id();
    let mirror = app.world_mut().spawn(Camera3d::default()).id();
    let _ = mirror;
    (app, cam)
}

#[test]
fn the_main_camera_gets_one_shared_cube_and_a_mirror_camera_does_not() {
    let (mut app, cam) = app_with_camera();
    let other = app
        .world_mut()
        .query_filtered::<Entity, (With<Camera3d>, Without<MainCamera>)>()
        .single(app.world())
        .unwrap();
    app.update();
    let g = app
        .world()
        .get::<GeneratedEnvironmentMapLight>(cam)
        .expect("light on the main camera");
    assert!(g.intensity > 100.0 && g.intensity <= BASE_INTENSITY);
    assert!(app
        .world()
        .get::<GeneratedEnvironmentMapLight>(other)
        .is_none());
    assert_eq!(app.world().resource::<SkyCubeState>().builds, 1);
    // Frames later without a change: no rebuild.
    for _ in 0..5 {
        app.update();
    }
    assert_eq!(app.world().resource::<SkyCubeState>().builds, 1);
}

#[test]
fn turning_glass_reflections_off_removes_everything() {
    let (mut app, cam) = app_with_camera();
    app.update();
    assert!(app
        .world()
        .get::<GeneratedEnvironmentMapLight>(cam)
        .is_some());
    app.world_mut()
        .resource_mut::<LightQuality>()
        .glass_reflections = false;
    app.update();
    assert!(app
        .world()
        .get::<GeneratedEnvironmentMapLight>(cam)
        .is_none());
    assert!(app.world().get::<SkyCubeApplied>(cam).is_none());
    assert!(app.world().resource::<SkyCubeState>().handle.is_none());
    app.world_mut()
        .resource_mut::<LightQuality>()
        .glass_reflections = true;
    app.update();
    assert!(app
        .world()
        .get::<GeneratedEnvironmentMapLight>(cam)
        .is_some());
}

#[test]
fn the_intensity_follows_the_day_without_a_rebuild() {
    let (mut app, cam) = app_with_camera();
    app.update();
    let day = app
        .world()
        .get::<GeneratedEnvironmentMapLight>(cam)
        .unwrap()
        .intensity;
    app.world_mut()
        .resource_mut::<SkyCubeInput>()
        .sun_elevation_deg = -30.0;
    app.update();
    let night = app
        .world()
        .get::<GeneratedEnvironmentMapLight>(cam)
        .unwrap()
        .intensity;
    assert!(night < day * 0.1, "night {night} day {day}");
    assert_eq!(
        app.world().resource::<SkyCubeState>().builds,
        1,
        "throttled: the clock did not advance"
    );
}
