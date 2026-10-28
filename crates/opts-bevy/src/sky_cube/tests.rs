//! Unit tests for `super` (`sky_cube.rs`).

use super::*;

fn input() -> SkyCubeInput {
    SkyCubeInput {
        horizon: [0.8, 0.85, 0.95],
        zenith: [0.2, 0.4, 0.8],
        sun_dir: Vec3::new(0.3, 0.6, 0.4).normalize(),
        sun_color: [1.0, 0.95, 0.8],
        sun_elevation_deg: 36.0,
        direct: 1.0,
        ambient: 1.0,
        fog: 0.0,
        revision: 1,
    }
}

fn snap(rev: u64, elev: f32, w: [f32; 3]) -> RegenSnapshot {
    RegenSnapshot {
        revision: rev,
        sun_elevation_deg: elev,
        weather: w,
    }
}

#[test]
fn the_first_cube_is_built_at_once() {
    assert!(should_regenerate(None, &snap(1, 10.0, [1.0; 3]), 0.0));
}

#[test]
fn nothing_changed_means_no_rebuild_ever() {
    let s = snap(1, 10.0, [1.0, 1.0, 0.0]);
    for t in [0.5, 2.0, 60.0, 3600.0] {
        assert!(!should_regenerate(Some((&s, 0.0)), &s, t));
    }
}

#[test]
fn a_change_waits_for_the_throttle() {
    let a = snap(1, 10.0, [1.0, 1.0, 0.0]);
    let b = snap(2, 10.0, [1.0, 1.0, 0.0]);
    assert!(!should_regenerate(Some((&a, 10.0)), &b, 11.9), "within 2 s");
    assert!(should_regenerate(Some((&a, 10.0)), &b, 12.0), "after 2 s");
}

#[test]
fn the_sun_must_move_more_than_two_degrees() {
    let a = snap(1, 10.0, [1.0; 3]);
    assert!(!should_regenerate(
        Some((&a, 0.0)),
        &snap(1, 11.9, [1.0; 3]),
        5.0
    ));
    assert!(!should_regenerate(
        Some((&a, 0.0)),
        &snap(1, 8.0, [1.0; 3]),
        5.0
    ));
    assert!(should_regenerate(
        Some((&a, 0.0)),
        &snap(1, 12.1, [1.0; 3]),
        5.0
    ));
    assert!(should_regenerate(
        Some((&a, 0.0)),
        &snap(1, 7.9, [1.0; 3]),
        5.0
    ));
}

#[test]
fn a_weather_change_rebuilds_each_component() {
    let a = snap(1, 10.0, [1.0, 1.0, 0.0]);
    assert!(!should_regenerate(
        Some((&a, 0.0)),
        &snap(1, 10.0, [0.99, 1.0, 0.0]),
        5.0
    ));
    for w in [[0.9, 1.0, 0.0], [1.0, 0.9, 0.0], [1.0, 1.0, 0.1]] {
        assert!(should_regenerate(Some((&a, 0.0)), &snap(1, 10.0, w), 5.0));
    }
}

#[test]
fn exposure_is_dark_at_night_and_scales_with_the_weather() {
    assert!((exposure(-30.0, 1.0) - NIGHT_EXPOSURE).abs() < 1e-6);
    assert!(exposure(-30.0, 1.0) < 0.05, "night does not glow");
    assert!((exposure(45.0, 1.0) - 1.0).abs() < 1e-6);
    assert!((exposure(45.0, 0.4) - 0.4).abs() < 1e-6, "overcast");
    assert!(exposure(0.0, 1.0) > exposure(-3.0, 1.0), "monotone at dawn");
    assert!(exposure(5.0, 1.0) < 1.0);
}

#[test]
fn the_sky_gets_brighter_towards_its_colours_and_the_sun_spot_is_there() {
    let i = input();
    let up = sky_radiance(Vec3::Y, &i);
    let hor = sky_radiance(Vec3::X, &i);
    assert!((up - Vec3::from(i.zenith)).length() < 0.05, "zenith {up:?}");
    assert!(
        (hor - Vec3::from(i.horizon)).length() < 0.05,
        "horizon {hor:?}"
    );
    let spot = sky_radiance(i.sun_dir, &i);
    assert!(spot.max_element() > 10.0, "sun spot {spot:?}");
    let down = sky_radiance(-Vec3::Y, &i);
    assert!(down.max_element() < hor.min_element(), "ground is darker");
}

#[test]
fn no_sun_spot_below_the_horizon() {
    let mut i = input();
    i.sun_elevation_deg = -20.0;
    i.sun_dir = Vec3::new(0.3, -0.6, 0.4).normalize();
    assert!(sky_radiance(i.sun_dir, &i).max_element() < 1.0);
}

#[test]
fn the_cube_data_has_six_faces_of_rgba16f() {
    let d = cube_data(&input(), 4);
    assert_eq!(d.len(), 4 * 4 * 6 * 8);
}

#[test]
fn face_directions_are_unit_and_cover_all_axes() {
    for face in 0..6 {
        let d = face_direction(face, 1, 2, 8);
        assert!((d.length() - 1.0).abs() < 1e-5);
    }
    assert!(face_direction(0, 4, 4, 8).x > 0.9);
    assert!(face_direction(1, 4, 4, 8).x < -0.9);
    assert!(face_direction(2, 4, 4, 8).y > 0.9);
    assert!(face_direction(3, 4, 4, 8).y < -0.9);
    assert!(face_direction(4, 4, 4, 8).z > 0.9);
    assert!(face_direction(5, 4, 4, 8).z < -0.9);
}

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
