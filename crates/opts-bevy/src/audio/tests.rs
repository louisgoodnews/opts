//! Engine tests without a sound card: voices are decided from positions, settings and the
//! budget; the players are Bevy components that stay unplayed without an audio plugin.

use bevy::audio::AudioPlayer;
use bevy::prelude::*;

use super::engine::choose_voices;
use super::*;
use crate::MainCamera;

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::transform::TransformPlugin,
    ))
    .add_plugins(OptsAudioPlugin);
    // Every bus at full level so the expected gains read simply (master stays 80).
    {
        let mut s = app.world_mut().resource_mut::<AudioSettings>();
        s.effects = 100.0;
        s.ambient = 100.0;
        s.vehicles = 100.0;
        s.ui = 100.0;
    }
    app.world_mut()
        .spawn((MainCamera, Transform::from_xyz(0.0, 0.0, 0.0)));
    app
}

fn clip(app: &mut App) -> Handle<PcmClip> {
    app.world_mut()
        .resource_mut::<Assets<PcmClip>>()
        .add(PcmClip::tone(220.0, 1.0, 8000))
}

fn emitter_at(app: &mut App, x: f32, e: SoundEmitter) -> Entity {
    app.world_mut()
        .spawn((e, Transform::from_xyz(x, 0.0, 0.0)))
        .id()
}

fn has_voice(app: &App, e: Entity) -> bool {
    app.world().get::<Voice>(e).is_some() && app.world().get::<AudioPlayer<PcmClip>>(e).is_some()
}

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

#[test]
fn one_shots_clean_up_after_themselves() {
    let mut app = app();
    let c = clip(&mut app);
    // Not heard when triggered: gone at once.
    let far = emitter_at(
        &mut app,
        900.0,
        SoundEmitter::new(c.clone(), SoundBus::Effects).spatial(Distance::new(5.0, 100.0)),
    );
    // Heard: plays, and goes when Bevy removes the finished player.
    let heard = emitter_at(
        &mut app,
        3.0,
        SoundEmitter::new(c, SoundBus::Effects).spatial(Distance::new(5.0, 100.0)),
    );
    app.update();
    app.update();
    assert!(app.world().get_entity(far).is_err());
    assert!(has_voice(&app, heard));
    app.world_mut()
        .entity_mut(heard)
        .remove::<AudioPlayer<PcmClip>>();
    app.update();
    app.update();
    assert!(app.world().get_entity(heard).is_err());
}

#[test]
fn settings_scale_and_silence_the_buses() {
    let mut app = app();
    let c = clip(&mut app);
    let e = emitter_at(
        &mut app,
        1.0,
        SoundEmitter::new(c, SoundBus::Vehicles).looping(),
    );
    app.world_mut().resource_mut::<AudioSettings>().vehicles = 50.0;
    app.update();
    app.update();
    let gain = app.world().get::<Voice>(e).map(|v| v.gain).unwrap_or(0.0);
    assert!((gain - 0.64 * 0.25).abs() < 1e-3, "{gain}");
    app.world_mut().resource_mut::<AudioSettings>().enabled = false;
    app.update();
    app.update();
    assert!(!has_voice(&app, e), "sound off: no voices");
    app.world_mut().resource_mut::<AudioSettings>().enabled = true;
    app.update();
    app.update();
    assert!(has_voice(&app, e));
}

#[test]
fn a_muted_source_is_not_heard_and_a_spatial_one_needs_a_listener() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::transform::TransformPlugin,
    ))
    .add_plugins(OptsAudioPlugin);
    let c = clip(&mut app);
    // No camera, so no listener: a spatial source is silent, a plain one is heard.
    let spatial = emitter_at(
        &mut app,
        1.0,
        SoundEmitter::new(c.clone(), SoundBus::Ambient)
            .looping()
            .spatial(Distance::default()),
    );
    let plain = app
        .world_mut()
        .spawn(SoundEmitter::new(c.clone(), SoundBus::Ui).looping())
        .id();
    let mut muted = SoundEmitter::new(c, SoundBus::Ui).looping();
    muted.muted = true;
    let muted = app.world_mut().spawn(muted).id();
    app.update();
    app.update();
    assert!(!has_voice(&app, spatial));
    assert!(has_voice(&app, plain));
    assert!(!has_voice(&app, muted));
}

#[test]
fn the_main_camera_becomes_the_listener() {
    let mut app = app();
    app.update();
    let mut q = app
        .world_mut()
        .query_filtered::<&bevy::audio::SpatialListener, With<MainCamera>>();
    assert_eq!(q.iter(app.world()).count(), 1);
}
