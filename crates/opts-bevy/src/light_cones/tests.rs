use std::time::Duration;

use bevy::time::TimeUpdateStrategy;

use super::shape::{
    cone_angles, headlight_cone_shape, lantern_cone_length, lantern_cone_shape,
    spotlight_cone_shape, ConeKind, ConeShape,
};
use super::*;
use crate::light_quality::{ladder_preset, LightMode, LightQualityInput};

/// The input that makes the effective quality exactly `q` (the plugin's resolver rewrites
/// `LightQuality` every frame from the input and the governor).
fn custom(q: LightQuality) -> LightQualityInput {
    LightQualityInput {
        mode: LightMode::Custom,
        custom: q,
    }
}

fn e(i: u32) -> Entity {
    Entity::from_raw_u32(i).expect("entity")
}

fn light(direction: Vec3, omni: bool, cone: bool, inner: f32, outer: f32) -> ScoLight {
    ScoLight {
        position: Vec3::new(1.0, 2.0, 7.0),
        direction,
        color: [255.0, 128.0, 0.0],
        size_m: 1.0,
        variable: "NightlightA".into(),
        factor: 1.0,
        omni,
        cone_inner_deg: inner,
        cone_outer_deg: outer,
        time_const_s: 0.0,
        volumetric_cone: cone,
        bitmap: String::new(),
        after_mesh: None,
        lod_threshold: None,
    }
}

// ---- mapping ------------------------------------------------------------------------------

mod mapping;

mod prepass;
use prepass::main_camera;

// ---- selection ----------------------------------------------------------------------------

mod selection;

// ---- the system in a headless app ---------------------------------------------------------

fn app(level: u8) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::transform::TransformPlugin))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            50,
        )))
        .insert_resource(custom(ladder_preset(level)))
        .add_plugins(LightConePlugin);
    app.world_mut().spawn((
        Camera::default(),
        crate::MainCamera,
        Transform::from_xyz(0.0, 1.7, 0.0),
    ));
    app
}

fn shape() -> ConeShape {
    ConeShape {
        apex: Vec3::ZERO,
        dir: Vec3::NEG_Y,
        length: 6.0,
        half_angle: 30f32.to_radians(),
        inner_frac: 0.3,
        color: [1.0; 3],
        ground_h: Some(5.0),
    }
}

/// A cone `z` metres in front of the camera.
fn spawn_cone(app: &mut App, kind: ConeKind, level: ConeLevel, z: f32) -> Entity {
    app.world_mut()
        .spawn((
            Transform::from_xyz(0.0, 5.0, -z),
            Visibility::Hidden,
            MeshTag::new(0),
            LightCone {
                kind,
                shape: shape(),
                level,
            },
        ))
        .id()
}

fn visible(app: &App, cones: &[Entity]) -> usize {
    cones
        .iter()
        .filter(|c| *app.world().get::<Visibility>(**c).unwrap() == Visibility::Inherited)
        .count()
}

fn run(app: &mut App, frames: usize) {
    for _ in 0..frames {
        app.update();
    }
}

fn spawn_mixed(app: &mut App) -> (Vec<Entity>, Vec<Entity>) {
    let lanterns = (1..=3)
        .map(|i| {
            spawn_cone(
                app,
                ConeKind::Lantern,
                ConeLevel::Fixed(1.0),
                10.0 * i as f32,
            )
        })
        .collect();
    let heads = (1..=3)
        .map(|i| {
            spawn_cone(
                app,
                ConeKind::Headlight,
                ConeLevel::Fixed(1.0),
                12.0 * i as f32,
            )
        })
        .collect();
    (lanterns, heads)
}

/// Ladder levels 0 to 5 show the expected cones: both kinds on levels 0 and 1, headlights only
/// on level 2, nothing from level 3 (cap 0 or both switches off).
#[test]
fn the_ladder_levels_show_the_expected_cones() {
    let expected = [
        (0, 3, 3),
        (1, 3, 3),
        (2, 0, 3),
        (3, 0, 0),
        (4, 0, 0),
        (5, 0, 0),
    ];
    for (level, lanterns, heads) in expected {
        let mut app = app(level);
        let (l, h) = spawn_mixed(&mut app);
        run(&mut app, 80);
        assert_eq!(visible(&app, &l), lanterns, "level {level} lanterns");
        assert_eq!(visible(&app, &h), heads, "level {level} headlights");
    }
}

/// A live change of the quality hides (or shows) cones at once, not after the next interval.
#[test]
fn a_live_ladder_change_applies_in_the_next_frame() {
    let mut app = app(0);
    let (l, h) = spawn_mixed(&mut app);
    run(&mut app, 80);
    assert_eq!(visible(&app, &l) + visible(&app, &h), 6);
    app.insert_resource(custom(ladder_preset(2)));
    run(&mut app, 1);
    assert_eq!((visible(&app, &l), visible(&app, &h)), (0, 3));
    app.insert_resource(custom(ladder_preset(5)));
    run(&mut app, 1);
    assert_eq!(visible(&app, &h), 0);
    app.insert_resource(custom(ladder_preset(0)));
    run(&mut app, 1);
    assert_eq!(
        visible(&app, &l) + visible(&app, &h),
        0,
        "the depth fade is back on: the cones wait for the prepass"
    );
    run(&mut app, 80);
    assert_eq!(visible(&app, &l) + visible(&app, &h), 6);
}

#[test]
fn the_cap_hides_the_farthest_cones_and_the_stats_count_them() {
    for (cap, spawned, shown) in [(0u32, 20usize, 0usize), (16, 40, 16), (96, 120, 96)] {
        let mut q = ladder_preset(0);
        q.cone_cap = cap;
        let mut app = app(0);
        app.insert_resource(custom(q));
        let cones: Vec<Entity> = (0..spawned)
            .map(|i| {
                spawn_cone(
                    &mut app,
                    ConeKind::Lantern,
                    ConeLevel::Fixed(1.0),
                    5.0 + i as f32,
                )
            })
            .collect();
        run(&mut app, 80);
        assert_eq!(visible(&app, &cones), shown, "cap {cap}");
        // The nearest ones are the visible ones.
        for c in &cones[..shown] {
            assert_eq!(
                *app.world().get::<Visibility>(*c).unwrap(),
                Visibility::Inherited
            );
        }
        let stats = *app.world().resource::<ConeStats>();
        assert_eq!(stats.total, spawned);
        assert_eq!(stats.active, shown);
        if cap > 0 {
            assert_eq!(stats.over_budget, spawned - shown);
        }
    }
}

#[test]
fn a_cone_that_is_not_lit_is_hidden_and_follows_its_light() {
    let mut app = app(0);
    let glow = app
        .world_mut()
        .spawn(SceneryLight {
            var: crate::scenery_runtime::VarRef::One,
            factor: 1.0,
            size_m: 1.0,
            direction: Vec3::ZERO,
            cos_inner: 1.0,
            cos_outer: 1.0,
            level: 0.0,
        })
        .id();
    let cone = spawn_cone(&mut app, ConeKind::Lantern, ConeLevel::Glow(glow), 10.0);
    run(&mut app, 80);
    assert_eq!(visible(&app, &[cone]), 0, "level 0 is hidden");
    // The lamp comes on: the next selection (within the interval) picks it up.
    app.world_mut().get_mut::<SceneryLight>(glow).unwrap().level = 0.5;
    run(&mut app, 80);
    assert_eq!(visible(&app, &[cone]), 1);
    let tag = app.world().get::<MeshTag>(cone).unwrap().value;
    assert_eq!(tag_level(tag), 32);
    // And off again at once.
    app.world_mut().get_mut::<SceneryLight>(glow).unwrap().level = 0.0;
    run(&mut app, 1);
    assert_eq!(visible(&app, &[cone]), 0);
}

#[test]
fn spot_cones_read_the_spotlights_intensity() {
    let mut app = app(0);
    let spot = app
        .world_mut()
        .spawn((
            SpotLight {
                intensity: 0.0,
                ..default()
            },
            ScriptLight {
                owner: Entity::PLACEHOLDER,
                switch: crate::vehicle_lights::LightSwitch::Spot(0),
                base_intensity: 100.0,
            },
        ))
        .id();
    let cone = spawn_cone(&mut app, ConeKind::Headlight, ConeLevel::Spot(spot), 10.0);
    run(&mut app, 80);
    assert_eq!(visible(&app, &[cone]), 0);
    app.world_mut()
        .get_mut::<SpotLight>(spot)
        .unwrap()
        .intensity = 100.0;
    run(&mut app, 80);
    assert_eq!(visible(&app, &[cone]), 1);
}

#[test]
fn cones_behind_the_camera_or_too_far_are_not_considered() {
    let mut app = app(0);
    let behind = spawn_cone(&mut app, ConeKind::Lantern, ConeLevel::Fixed(1.0), -40.0);
    let far = spawn_cone(
        &mut app,
        ConeKind::Lantern,
        ConeLevel::Fixed(1.0),
        LANTERN_MAX_DISTANCE_M + 100.0,
    );
    // A headlight reaches farther than a lantern.
    let far_head = spawn_cone(
        &mut app,
        ConeKind::Headlight,
        ConeLevel::Fixed(1.0),
        LANTERN_MAX_DISTANCE_M + 100.0,
    );
    run(&mut app, 80);
    assert_eq!(visible(&app, &[behind, far]), 0);
    assert_eq!(visible(&app, &[far_head]), 1);
}

// ---- depth prepass ------------------------------------------------------------------------

// ---- cost ---------------------------------------------------------------------------------

/// 2000 lamps with cones (about as many as the map has), all lit: the whole app update stays
/// small (a generous bound for unoptimised test builds; the release figure is in the design
/// note).
#[test]
fn the_cpu_cost_with_two_thousand_lamps_is_small() {
    let mut app = app(0);
    for i in 0..2000 {
        let x = (i % 50) as f32 * 8.0 - 200.0;
        let z = (i / 50) as f32 * 8.0 - 150.0;
        app.world_mut().spawn((
            Transform::from_xyz(x, 6.0, z),
            Visibility::Hidden,
            MeshTag::new(0),
            LightCone {
                kind: ConeKind::Lantern,
                shape: shape(),
                level: ConeLevel::Fixed(1.0),
            },
        ));
    }
    run(&mut app, 80);
    let stats = *app.world().resource::<ConeStats>();
    assert_eq!(stats.total, 2000);
    assert_eq!(stats.active, 64);
    let started = std::time::Instant::now();
    run(&mut app, 100);
    let per_update = started.elapsed() / 100;
    eprintln!(
        "cones: 2000 lamps, update {:?} average (whole app), last selection {} us",
        per_update,
        app.world().resource::<ConeStats>().select_us
    );
    assert!(per_update < Duration::from_millis(20), "{per_update:?}");
}

/// While the simulation is paused (the Environment page previews times) the cone pool and the
/// depth-fade gate still run on real time and follow the lamp's level when it changes.
#[test]
fn cones_follow_their_light_while_the_virtual_clock_is_paused() {
    let mut app = app(0);
    let glow = app
        .world_mut()
        .spawn(SceneryLight {
            var: crate::scenery_runtime::VarRef::One,
            factor: 1.0,
            size_m: 1.0,
            direction: Vec3::ZERO,
            cos_inner: 1.0,
            cos_outer: 1.0,
            level: 0.0,
        })
        .id();
    let cone = spawn_cone(&mut app, ConeKind::Lantern, ConeLevel::Glow(glow), 10.0);
    run(&mut app, 4);
    app.world_mut().resource_mut::<Time<Virtual>>().pause();
    app.world_mut().get_mut::<SceneryLight>(glow).unwrap().level = 1.0;
    run(&mut app, 100);
    assert_eq!(app.world().resource::<ConeStats>().active, 1);
    assert_eq!(
        visible(&app, &[cone]),
        1,
        "selected, gated and faded in on real time"
    );
    let camera = main_camera(&mut app);
    assert!(app.world().get::<DepthPrepass>(camera).is_some());
    app.world_mut().get_mut::<SceneryLight>(glow).unwrap().level = 0.0;
    run(&mut app, 1);
    assert_eq!(visible(&app, &[cone]), 0);
}
