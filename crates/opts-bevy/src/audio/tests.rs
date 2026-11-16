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
mod buses;
mod listener;
mod voices;
