//! The cost of the cone system with many lamps, and the pause of the virtual clock.

use super::*;

/// 2000 lamps with cones (about as many as the map has), all lit: the whole app update stays
/// small (a generous bound for unoptimised test builds; the release figure is in the design
/// note).
#[test]
pub(super) fn the_cpu_cost_with_two_thousand_lamps_is_small() {
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
pub(super) fn cones_follow_their_light_while_the_virtual_clock_is_paused() {
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
