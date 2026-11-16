//! The lamp: the script switches it by time of day, its light sprite hangs under the root and it follows the night switch while paused.

use super::*;

#[test]
fn the_script_switches_the_lamp_by_time_of_day() {
    let mut app = app();
    host_driven(&mut app, 12.0);
    advance_scenery(app.world_mut(), 1.0 / 60.0);
    let rt = runtime(&app);
    let (glass, light) = (rt.meshes[0], rt.lights[0]);
    assert_eq!(visibility(&app, glass), Visibility::Hidden);
    assert_eq!(app.world().get::<SceneryLight>(light).unwrap().level, 0.0);

    host_driven(&mut app, 23.0);
    advance_scenery(app.world_mut(), 1.0 / 60.0);
    assert_eq!(visibility(&app, glass), Visibility::Inherited);
    assert_eq!(app.world().get::<SceneryLight>(light).unwrap().level, 1.0);
    // The sprite of a lit, omnidirectional light shows after the frame's light update.
    app.update();
    assert_eq!(visibility(&app, light), Visibility::Inherited);
}

/// The light is declared inside the `[LOD] 0.2` band but hangs under the object root, so a LOD
/// switch never hides the glow.
#[test]
fn the_light_sprite_hangs_under_the_root_not_a_lod_group() {
    let app = app();
    let root = app.world().resource::<Root>().0;
    let light = runtime(&app).lights[0];
    let parent = app.world().get::<ChildOf>(light).expect("parent").parent();
    assert_eq!(parent, root);
    let lod = app.world().get::<SceneryLod>(root).expect("LOD groups");
    assert!(!lod.groups.iter().any(|(g, _)| *g == parent));
}

/// While the simulation clock is paused (the Environment page previews the time), a switch to
/// night still reaches the lamps: the lights follow the host, no script time advances.
#[test]
fn lights_follow_the_night_switch_while_the_clock_is_paused() {
    let mut app = app();
    app.insert_resource(SceneryClock::at(2026, 6, 29, 12.0 * 3600.0));
    for _ in 0..3 {
        app.update();
    }
    let light = runtime(&app).lights[0];
    let level = |app: &App| app.world().get::<SceneryLight>(light).unwrap().level;
    assert_eq!(level(&app), 0.0, "day: the lamp is off");
    app.world_mut().resource_mut::<Time<Virtual>>().pause();
    app.insert_resource(SceneryClock::at(2026, 6, 29, 22.0 * 3600.0));
    for _ in 0..3 {
        app.update();
    }
    assert!(level(&app) > 0.5, "night while paused: {}", level(&app));
}
