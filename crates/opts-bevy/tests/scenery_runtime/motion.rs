//! The animated mesh and the runtime ticking from Bevy time.

use super::*;

#[test]
fn the_animated_mesh_moves_with_simulated_time() {
    let mut app = app();
    host_driven(&mut app, 12.0);
    let hand = runtime(&app).meshes[1];
    for _ in 0..60 {
        advance_scenery(app.world_mut(), 1.0 / 60.0);
    }
    let after_1s = *app.world().get::<Transform>(hand).unwrap();
    for _ in 0..60 {
        advance_scenery(app.world_mut(), 1.0 / 60.0);
    }
    let after_2s = *app.world().get::<Transform>(hand).unwrap();
    let angle = |t: &Transform| t.rotation.angle_between(Quat::IDENTITY).to_degrees();
    assert!((angle(&after_1s) - 10.0).abs() < 0.5, "{after_1s:?}");
    assert!((angle(&after_2s) - 20.0).abs() < 0.5, "{after_2s:?}");
    let root = app.world().resource::<Root>().0;
    let scripts = app.world().get::<SceneryScripts>(root).unwrap();
    // 120 steps plus the initial evaluation of the first, zero-length frame (it advances no
    // script time: the angles above are exact).
    assert_eq!(scripts.ticks, 121);
    let deg = scripts.with_engine(|e| e.get("hand_deg")).unwrap();
    assert!((deg - 20.0).abs() < 1e-3, "{deg}");
    let clock = app.world().resource::<SceneryClock>().seconds_of_day;
    assert!((clock - (12.0 * 3600.0 + 2.0)).abs() < 1e-4, "{clock}");
}

#[test]
fn the_runtime_ticks_by_itself_from_bevy_time() {
    let mut app = app();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        100,
    )));
    for _ in 0..11 {
        app.update();
    }
    let root = app.world().resource::<Root>().0;
    let deg = app
        .world()
        .get::<SceneryScripts>(root)
        .unwrap()
        .with_engine(|e| e.get("hand_deg"))
        .unwrap();
    assert!(deg > 5.0, "the near object ticks every frame: {deg}");
    let stats = app.world().resource::<SceneryRuntimeStats>();
    assert_eq!((stats.objects, stats.scripted), (1, 1));
}
