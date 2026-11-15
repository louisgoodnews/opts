//! Cones and the lights they follow: intensity, visibility and the camera's view.

use super::*;

#[test]
pub(super) fn a_cone_that_is_not_lit_is_hidden_and_follows_its_light() {
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
pub(super) fn spot_cones_read_the_spotlights_intensity() {
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
pub(super) fn cones_behind_the_camera_or_too_far_are_not_considered() {
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
