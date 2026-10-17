use std::time::Duration;

use bevy::time::TimeUpdateStrategy;

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

#[test]
fn only_directional_lights_with_the_cone_flag_get_a_lantern_cone() {
    let down = Vec3::new(0.0, 0.0, -1.0);
    assert!(lantern_cone_shape(&light(down, false, true, 120.0, 200.0)).is_some());
    assert!(
        lantern_cone_shape(&light(down, true, true, 120.0, 200.0)).is_none(),
        "omni"
    );
    assert!(
        lantern_cone_shape(&light(down, false, false, 20.0, 40.0)).is_none(),
        "a narrow signal lamp without the flag"
    );
    assert!(
        lantern_cone_shape(&light(Vec3::ZERO, false, true, 20.0, 40.0)).is_none(),
        "no direction"
    );
}

#[test]
fn a_lantern_cone_maps_position_direction_angles_length_and_colour() {
    // OMSI (1, 2, 7) is Bevy (1, 7, -2); OMSI down (0, 0, -1) is Bevy (0, -1, 0).
    let s = lantern_cone_shape(&light(Vec3::new(0.0, 0.0, -1.0), false, true, 120.0, 200.0))
        .expect("cone");
    assert_eq!(s.apex, Vec3::new(1.0, 7.0, -2.0));
    assert!((s.dir - Vec3::NEG_Y).length() < 1e-6);
    // 200 degrees is clamped to the 40 degree half angle; the inner edge keeps the 120/200 ratio.
    assert!((s.half_angle - 40f32.to_radians()).abs() < 1e-6);
    let want = (40f32.to_radians() * 0.6).tan() / 40f32.to_radians().tan();
    assert!((s.inner_frac - want).abs() < 1e-5, "{}", s.inner_frac);
    // Pointing down from 7 m: the length is the way to the ground.
    assert!((s.length - 7.0).abs() < 1e-5);
    assert_eq!(s.color, [1.0, 128.0 / 255.0, 0.0]);
    // The transform maps the unit cone's -Z onto the direction and scales radius and length.
    let t = s.transform();
    assert_eq!(t.translation, s.apex);
    assert!((t.rotation * Vec3::NEG_Z - Vec3::NEG_Y).length() < 1e-5);
    assert!((t.scale.z - 7.0).abs() < 1e-5);
    assert!((t.scale.x - 7.0 * 40f32.to_radians().tan()).abs() < 1e-4);
}

#[test]
fn lantern_length_falls_back_when_it_does_not_point_down() {
    assert_eq!(
        lantern_cone_length(Vec3::new(0.0, 7.0, 0.0), Vec3::X),
        LANTERN_DEFAULT_LENGTH_M
    );
    // Slanted: the way along the axis to the ground, clamped.
    let d = Vec3::new(1.0, -1.0, 0.0).normalize();
    assert!((lantern_cone_length(Vec3::new(0.0, 4.0, 0.0), d) - 4.0 * 2f32.sqrt()).abs() < 1e-4);
    assert_eq!(
        lantern_cone_length(Vec3::new(0.0, 100.0, 0.0), Vec3::NEG_Y),
        LANTERN_LENGTH_M.1
    );
    assert_eq!(
        lantern_cone_length(Vec3::new(0.0, 0.5, 0.0), Vec3::NEG_Y),
        LANTERN_LENGTH_M.0
    );
}

#[test]
fn cone_angles_keep_the_ratio_and_the_limits() {
    let (half, inner) = cone_angles(12.0, 30.0, HEADLIGHT_HALF_DEG);
    assert!((half - 15f32.to_radians()).abs() < 1e-6);
    let want = (15f32.to_radians() * 0.4).tan() / 15f32.to_radians().tan();
    assert!((inner - want).abs() < 1e-5);
    // Too narrow and too wide are clamped; no outer angle gives the widest.
    assert!((cone_angles(0.0, 4.0, HEADLIGHT_HALF_DEG).0 - 6f32.to_radians()).abs() < 1e-6);
    assert!((cone_angles(0.0, 190.0, HEADLIGHT_HALF_DEG).0 - 25f32.to_radians()).abs() < 1e-6);
    assert!((cone_angles(0.0, 0.0, HEADLIGHT_HALF_DEG).0 - 25f32.to_radians()).abs() < 1e-6);
    // Inner larger than outer is held below the outer edge.
    assert!(cone_angles(80.0, 30.0, HEADLIGHT_HALF_DEG).1 <= 0.95);
}

#[test]
fn a_headlight_cone_follows_the_light_and_a_spotlight_cone_its_range() {
    // OMSI forward (+y) is Bevy -Z.
    let h = light(Vec3::Y, false, false, 150.0, 190.0);
    let s = headlight_cone_shape(&h).expect("headlight");
    assert!((s.dir - Vec3::NEG_Z).length() < 1e-6);
    assert_eq!(s.length, HEADLIGHT_CONE_LENGTH_M);
    assert!((s.half_angle - 25f32.to_radians()).abs() < 1e-6);
    assert!(headlight_cone_shape(&light(Vec3::ZERO, false, false, 1.0, 2.0)).is_none());

    let spot = Spotlight {
        pos: Vec3::new(0.5, 3.0, 0.7),
        dir: Vec3::Y,
        color_rgb: [255, 245, 215],
        range: 500.0,
        inner_cone: 30.0,
        outer_cone: 70.0,
    };
    let c = spotlight_cone_shape(&spot);
    // In the SpotLight entity's frame: apex at the origin, along -Z, range capped at 40 m.
    assert_eq!((c.apex, c.dir), (Vec3::ZERO, Vec3::NEG_Z));
    assert_eq!(c.length, SPOT_CONE_MAX_LENGTH_M);
    assert!(
        (c.half_angle - 25f32.to_radians()).abs() < 1e-6,
        "35 degrees clamped"
    );
    let near = Spotlight {
        range: 12.0,
        ..spot
    };
    assert_eq!(spotlight_cone_shape(&near).length, 12.0);
    assert_eq!(c.transform().rotation, Quat::IDENTITY);
}

#[test]
fn the_tag_packs_colour_level_and_inner_edge() {
    let tag = pack_tag([1.0, 0.5, 0.0], 0.5, 0.25);
    assert_eq!(tag & 63, 63);
    assert_eq!((tag >> 6) & 63, 32);
    assert_eq!((tag >> 12) & 63, 0);
    assert_eq!(tag_level(tag), 128);
    assert_eq!(tag >> 26, 16);
    assert_eq!(
        tag_level(pack_tag([0.0; 3], 2.0, 0.0)),
        255,
        "level is clamped"
    );
    assert_eq!(tag_level(pack_tag([0.0; 3], -1.0, 0.0)), 0);
}

// ---- selection ----------------------------------------------------------------------------

fn candidates(n: u32) -> Vec<(Entity, f32)> {
    // Coverage falls with the index: cone 1 is the biggest.
    (1..=n).map(|i| (e(i), 1000.0 / i as f32)).collect()
}

#[test]
fn the_cap_limits_the_selection() {
    let c = candidates(200);
    assert!(select_cones(&c, &[], 0, CONE_HYSTERESIS).is_empty());
    let sixteen = select_cones(&c, &[], 16, CONE_HYSTERESIS);
    assert_eq!(sixteen, (1..=16).map(e).collect::<Vec<_>>());
    let all = select_cones(&c, &[], 96, CONE_HYSTERESIS);
    assert_eq!(all.len(), 96);
    assert_eq!(all[0], e(1));
    assert_eq!(all[95], e(96));
    // Fewer candidates than the cap.
    assert_eq!(select_cones(&c[..5], &[], 96, CONE_HYSTERESIS).len(), 5);
}

#[test]
fn a_cone_that_holds_a_slot_keeps_it_against_a_slightly_bigger_one() {
    // Cone 3 is the fourth by coverage, but holds a slot; cone 4 is 10 % bigger.
    let c = [(e(1), 10.0), (e(2), 9.0), (e(3), 8.0), (e(4), 8.8)];
    let first = select_cones(&c, &[e(3)], 3, CONE_HYSTERESIS);
    assert!(first.contains(&e(3)) && !first.contains(&e(4)));
    // Without the slot it would lose.
    assert!(!select_cones(&c, &[], 3, CONE_HYSTERESIS).contains(&e(3)));
    // A much bigger newcomer still wins.
    let c = [(e(1), 10.0), (e(2), 9.0), (e(3), 8.0), (e(4), 20.0)];
    let next = select_cones(&c, &[e(3)], 3, CONE_HYSTERESIS);
    assert!(next.contains(&e(4)));
    // Repeating the choice gives the same set.
    let mut a = select_cones(&c, &next, 3, CONE_HYSTERESIS);
    a.sort();
    let mut b = next;
    b.sort();
    assert_eq!(a, b);
}

#[test]
fn coverage_grows_with_size_and_falls_with_distance() {
    assert!(cone_coverage(8.0, 4.0, 10.0) > cone_coverage(8.0, 4.0, 20.0));
    assert!(cone_coverage(30.0, 12.0, 50.0) > cone_coverage(8.0, 4.0, 50.0));
    assert!(cone_coverage(8.0, 4.0, 0.0).is_finite());
}

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
        run(&mut app, 6);
        assert_eq!(visible(&app, &l), lanterns, "level {level} lanterns");
        assert_eq!(visible(&app, &h), heads, "level {level} headlights");
    }
}

/// A live change of the quality hides (or shows) cones at once, not after the next interval.
#[test]
fn a_live_ladder_change_applies_in_the_next_frame() {
    let mut app = app(0);
    let (l, h) = spawn_mixed(&mut app);
    run(&mut app, 6);
    assert_eq!(visible(&app, &l) + visible(&app, &h), 6);
    app.insert_resource(custom(ladder_preset(2)));
    run(&mut app, 1);
    assert_eq!((visible(&app, &l), visible(&app, &h)), (0, 3));
    app.insert_resource(custom(ladder_preset(5)));
    run(&mut app, 1);
    assert_eq!(visible(&app, &h), 0);
    app.insert_resource(custom(ladder_preset(0)));
    run(&mut app, 1);
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
        run(&mut app, 6);
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
    run(&mut app, 6);
    assert_eq!(visible(&app, &[cone]), 0, "level 0 is hidden");
    // The lamp comes on: the next selection (within the interval) picks it up.
    app.world_mut().get_mut::<SceneryLight>(glow).unwrap().level = 0.5;
    run(&mut app, 5);
    assert_eq!(visible(&app, &[cone]), 1);
    let tag = app.world().get::<MeshTag>(cone).unwrap().value;
    assert_eq!(tag_level(tag), 128);
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
    run(&mut app, 6);
    assert_eq!(visible(&app, &[cone]), 0);
    app.world_mut()
        .get_mut::<SpotLight>(spot)
        .unwrap()
        .intensity = 100.0;
    run(&mut app, 5);
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
    run(&mut app, 6);
    assert_eq!(visible(&app, &[behind, far]), 0);
    assert_eq!(visible(&app, &[far_head]), 1);
}

// ---- depth prepass ------------------------------------------------------------------------

#[test]
fn the_prepass_rule() {
    let q = ladder_preset(0);
    assert!(wants_depth_prepass(&q, 1, 0.0, false));
    assert!(!wants_depth_prepass(&q, 0, 0.0, false), "no cone shines");
    assert!(wants_depth_prepass(&q, 0, 5.0, true), "kept briefly");
    assert!(!wants_depth_prepass(&q, 0, PREPASS_RELEASE_S + 1.0, true));
    let mut off = q;
    off.cone_depth_fade = false;
    assert!(!wants_depth_prepass(&off, 3, 0.0, true));
    let mut none = q;
    none.cone_cap = 0;
    assert!(!wants_depth_prepass(&none, 3, 0.0, true));
    let mut no_kind = q;
    (no_kind.lantern_cones, no_kind.headlight_cones) = (false, false);
    assert!(!wants_depth_prepass(&no_kind, 3, 0.0, true));
}

fn main_camera(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<crate::MainCamera>>()
        .single(app.world())
        .expect("camera")
}

#[test]
fn the_depth_fade_setting_adds_and_removes_the_prepass() {
    let mut app = app(0);
    let (l, _) = spawn_mixed(&mut app);
    let camera = main_camera(&mut app);
    run(&mut app, 6);
    assert!(visible(&app, &l) > 0);
    assert!(app.world().get::<DepthPrepass>(camera).is_some());
    assert!(app.world().get::<ConeDepthPrepass>(camera).is_some());
    // Depth fade off (level 2 and up, or the custom control): the prepass goes at once.
    let mut q = ladder_preset(0);
    q.cone_depth_fade = false;
    app.insert_resource(custom(q));
    run(&mut app, 2);
    assert!(app.world().get::<DepthPrepass>(camera).is_none());
    assert!(app.world().get::<ConeDepthPrepass>(camera).is_none());
    // And on again.
    app.insert_resource(custom(ladder_preset(0)));
    run(&mut app, 2);
    assert!(app.world().get::<DepthPrepass>(camera).is_some());
}

#[test]
fn a_prepass_somebody_else_added_is_left_alone() {
    let mut app = app(2);
    let camera = main_camera(&mut app);
    app.world_mut().entity_mut(camera).insert(DepthPrepass);
    spawn_mixed(&mut app);
    run(&mut app, 6);
    assert!(app.world().get::<DepthPrepass>(camera).is_some());
    assert!(app.world().get::<ConeDepthPrepass>(camera).is_none());
}

#[test]
fn the_prepass_goes_when_the_last_cone_has_been_out_for_a_while() {
    let mut app = app(0);
    let cone = spawn_cone(&mut app, ConeKind::Lantern, ConeLevel::Fixed(1.0), 10.0);
    let camera = main_camera(&mut app);
    run(&mut app, 6);
    assert!(app.world().get::<DepthPrepass>(camera).is_some());
    app.world_mut().entity_mut(cone).insert(LightCone {
        kind: ConeKind::Lantern,
        shape: shape(),
        level: ConeLevel::Fixed(0.0),
    });
    run(&mut app, 6);
    assert!(
        app.world().get::<DepthPrepass>(camera).is_some(),
        "kept for a while"
    );
    // 50 ms per frame: 20 s is 400 frames.
    run(&mut app, 420);
    assert!(app.world().get::<DepthPrepass>(camera).is_none());
}

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
    run(&mut app, 6);
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
