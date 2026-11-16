//! Voice tests: the budget, the near and far sources, and the loop that loses and regains its voice.

use super::*;


#[test]
fn the_budget_keeps_the_loudest_and_drops_the_inaudible() {
    let levels = [(0, 0.2), (1, 0.9), (2, 0.0005), (3, 0.5)];
    assert_eq!(choose_voices(&levels, 2), vec![1, 3]);
    assert_eq!(choose_voices(&levels, 10), vec![1, 3, 0]);
    assert!(choose_voices(&levels, 0).is_empty());
    // Equal levels keep their order.
    assert_eq!(choose_voices(&[(5, 0.4), (2, 0.4)], 1), vec![2]);
}

#[test]
fn a_near_source_gets_a_voice_a_far_one_does_not() {
    let mut app = app();
    let c = clip(&mut app);
    let near = emitter_at(
        &mut app,
        10.0,
        SoundEmitter::new(c.clone(), SoundBus::Ambient)
            .looping()
            .spatial(Distance::new(5.0, 100.0)),
    );
    let far = emitter_at(
        &mut app,
        400.0,
        SoundEmitter::new(c, SoundBus::Ambient)
            .looping()
            .spatial(Distance::new(5.0, 100.0)),
    );
    app.update();
    app.update();
    assert!(has_voice(&app, near));
    assert!(!has_voice(&app, far), "beyond its range it costs nothing");
    // Looping sources stay when they are not heard.
    assert!(app.world().get_entity(far).is_ok());
    let stats = app.world().resource::<AudioStats>();
    assert_eq!((stats.emitters, stats.audible, stats.voices), (2, 1, 1));
    // The level is source × distance fade × bus × master: 0.8² (master) × 1 (bus 100) × 0.5.
    let gain = app
        .world()
        .get::<Voice>(near)
        .map(|v| v.gain)
        .unwrap_or(0.0);
    assert!((gain - 0.64 * 0.5).abs() < 1e-3, "{gain}");
}

#[test]
fn a_loop_loses_its_voice_out_of_range_and_gets_it_back() {
    let mut app = app();
    let c = clip(&mut app);
    let e = emitter_at(
        &mut app,
        10.0,
        SoundEmitter::new(c, SoundBus::Vehicles)
            .looping()
            .spatial(Distance::new(5.0, 100.0)),
    );
    app.update();
    app.update();
    assert!(has_voice(&app, e));
    app.world_mut()
        .entity_mut(e)
        .insert(Transform::from_xyz(500.0, 0.0, 0.0));
    app.update();
    app.update();
    assert!(!has_voice(&app, e));
    assert!(app.world().get::<AudioPlayer<PcmClip>>(e).is_none());
    app.world_mut()
        .entity_mut(e)
        .insert(Transform::from_xyz(20.0, 0.0, 0.0));
    app.update();
    app.update();
    assert!(has_voice(&app, e));
}

#[test]
fn the_voice_budget_applies() {
    let mut app = app();
    app.world_mut().resource_mut::<AudioSettings>().max_voices = 2;
    let c = clip(&mut app);
    let ids: Vec<Entity> = [10.0, 30.0, 60.0]
        .iter()
        .map(|x| {
            emitter_at(
                &mut app,
                *x,
                SoundEmitter::new(c.clone(), SoundBus::Ambient)
                    .looping()
                    .spatial(Distance::new(5.0, 200.0)),
            )
        })
        .collect();
    app.update();
    app.update();
    assert!(has_voice(&app, ids[0]) && has_voice(&app, ids[1]));
    assert!(!has_voice(&app, ids[2]), "the farthest is left out");
    let stats = app.world().resource::<AudioStats>();
    assert_eq!((stats.audible, stats.voices, stats.virtualized), (3, 2, 1));
}
