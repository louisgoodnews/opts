//! Listener tests: the muted and spatial sources, and the main camera as the listener.

use super::*;

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
