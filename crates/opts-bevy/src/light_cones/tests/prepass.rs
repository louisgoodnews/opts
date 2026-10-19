//! The depth prepass gate and its rules.

use super::*;

#[test]
fn cones_wait_for_the_prepass_and_fade_in_when_the_depth_fade_turns_on() {
    let mut q = ladder_preset(0);
    q.cone_depth_fade = false;
    let mut app = app(0);
    app.insert_resource(custom(q));
    let cone = spawn_cone(&mut app, ConeKind::Lantern, ConeLevel::Fixed(1.0), 10.0);
    run(&mut app, 10);
    assert_eq!(visible(&app, &[cone]), 1, "no fade, no gate");
    app.insert_resource(custom(ladder_preset(0)));
    run(&mut app, 5);
    assert_eq!(visible(&app, &[cone]), 0, "hidden while the prepass is new");
    assert_eq!(
        app.world().resource::<ConeStats>().active,
        1,
        "still wanted"
    );
    run(&mut app, 50);
    assert_eq!(visible(&app, &[cone]), 0, "2.5 s: not yet");
    run(&mut app, 20);
    assert_eq!(visible(&app, &[cone]), 1);
    // And it fades in rather than popping.
    let level = tag_level(app.world().get::<MeshTag>(cone).unwrap().value);
    assert!(level > 0);
    // Turning the fade off shows it at once and drops the prepass.
    app.insert_resource(custom(q));
    run(&mut app, 2);
    assert_eq!(visible(&app, &[cone]), 1);
}

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

pub(super) fn main_camera(app: &mut App) -> Entity {
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
    run(&mut app, 80);
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
    run(&mut app, 80);
    assert!(app.world().get::<DepthPrepass>(camera).is_some());
    assert!(app.world().get::<ConeDepthPrepass>(camera).is_none());
}

#[test]
fn the_prepass_goes_when_the_last_cone_has_been_out_for_a_while() {
    let mut app = app(0);
    let cone = spawn_cone(&mut app, ConeKind::Lantern, ConeLevel::Fixed(1.0), 10.0);
    let camera = main_camera(&mut app);
    run(&mut app, 80);
    assert!(app.world().get::<DepthPrepass>(camera).is_some());
    app.world_mut().entity_mut(cone).insert(LightCone {
        kind: ConeKind::Lantern,
        shape: shape(),
        level: ConeLevel::Fixed(0.0),
    });
    run(&mut app, 80);
    assert!(
        app.world().get::<DepthPrepass>(camera).is_some(),
        "kept for a while"
    );
    // 50 ms per frame: 20 s is 400 frames.
    run(&mut app, 420);
    assert!(app.world().get::<DepthPrepass>(camera).is_none());
}
