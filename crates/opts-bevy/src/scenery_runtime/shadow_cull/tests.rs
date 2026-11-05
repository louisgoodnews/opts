//! Tests of the per-part shadow culling (M-0029).

use std::time::Duration;

use bevy::camera::primitives::Aabb;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;

use super::*;

#[test]
fn small_parts_stop_casting_first_and_borders_do_not_toggle() {
    // k = 100: a 0.3 m part casts to 30 m, a 3 m part to 300 m.
    assert!(!is_shadow_culled(29.0, 0.3, 100.0, false));
    assert!(is_shadow_culled(31.0, 0.3, 100.0, false));
    assert!(!is_shadow_culled(250.0, 3.0, 100.0, false));
    // A culled part casts again only once it is 10 % inside.
    assert!(is_shadow_culled(29.0, 0.3, 100.0, true));
    assert!(!is_shadow_culled(26.0, 0.3, 100.0, true));
}

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<ShadowDetail>()
        .add_systems(
            Update,
            (mark_shadow_candidates, scenery_shadow_cull_system).chain(),
        );
    app.world_mut().spawn((
        Camera::default(),
        crate::MainCamera,
        GlobalTransform::default(),
    ));
    app
}

/// A scenery object root at `x` with one mesh part of half-extent `half` (m).
fn part(app: &mut App, x: f32, half: f32) -> Entity {
    let world = app.world_mut();
    let root = world
        .spawn((SceneryDrawRoot, GlobalTransform::from_xyz(x, 0.0, 0.0)))
        .id();
    world
        .spawn((
            Mesh3d(Handle::default()),
            Aabb::from_min_max(Vec3::splat(-half), Vec3::splat(half)),
            GlobalTransform::from_xyz(x, 0.0, 0.0),
            ChildOf(root),
        ))
        .id()
}

fn pass(app: &mut App) {
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_secs(1));
    app.update();
}

fn casts(app: &App, e: Entity) -> bool {
    app.world().get::<NotShadowCaster>(e).is_none()
}

#[test]
fn a_far_small_part_stops_casting_and_casts_again_when_switched_off() {
    let mut app = app();
    // Radius of a cube of half-extent h is h·√3: 0.2 m → 0.35 m, 2 m → 3.5 m.
    let small = part(&mut app, 80.0, 0.2);
    let large = part(&mut app, 80.0, 2.0);
    pass(&mut app);
    assert!(app.world().get::<ShadowCullCandidate>(small).is_some());
    assert!(casts(&app, small) && casts(&app, large), "off by default");

    app.world_mut().resource_mut::<ShadowDetail>().factor = Some(100.0);
    pass(&mut app);
    assert!(!casts(&app, small), "35 m reach at 80 m");
    assert!(casts(&app, large), "350 m reach at 80 m");

    app.world_mut().resource_mut::<ShadowDetail>().factor = None;
    pass(&mut app);
    assert!(casts(&app, small));
    assert!(app.world().get::<ShadowCulled>(small).is_none());
}

#[test]
fn effects_and_meshes_outside_scenery_are_left_alone() {
    let mut app = app();
    let effect = part(&mut app, 500.0, 0.1);
    app.world_mut().entity_mut(effect).insert(NotShadowCaster);
    let loose = app
        .world_mut()
        .spawn((
            Mesh3d(Handle::default()),
            Aabb::from_min_max(Vec3::splat(-0.1), Vec3::splat(0.1)),
            GlobalTransform::from_xyz(500.0, 0.0, 0.0),
        ))
        .id();
    app.world_mut().resource_mut::<ShadowDetail>().factor = Some(100.0);
    pass(&mut app);
    pass(&mut app);
    assert!(app.world().get::<ShadowCullCandidate>(effect).is_none());
    assert!(app.world().get::<ShadowCullCandidate>(loose).is_none());
    assert!(casts(&app, loose));
    // Switching off must not make the effect a caster.
    app.world_mut().resource_mut::<ShadowDetail>().factor = None;
    pass(&mut app);
    assert!(!casts(&app, effect));
}
