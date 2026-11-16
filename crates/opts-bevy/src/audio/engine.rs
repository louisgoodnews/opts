//! The sound engine's systems: listener, voice selection, level sync and the statistics the
//! control protocol reports (`audio.state`).

use bevy::audio::{
    AudioPlayer, AudioSink, AudioSinkPlayback, PlaybackMode, PlaybackSettings, SpatialAudioSink,
    SpatialListener, SpatialScale, Volume,
};
use bevy::prelude::*;

use super::clip::PcmClip;
use super::curves::percent_to_gain;
use super::emitter::{SoundBus, SoundEmitter, Voice};
use crate::MainCamera;
mod select;
mod settings;
mod stats;

pub use select::*;
pub use settings::*;
pub use stats::*;

/// Levels below this are not worth a voice.
pub const AUDIBLE_MIN: f32 = 0.002;

/// Puts a [`SpatialListener`] on the main camera (a head about 0.2 m wide).
pub fn ensure_listener(
    cameras: Query<Entity, (With<MainCamera>, Without<SpatialListener>)>,
    mut commands: Commands,
) {
    for e in &cameras {
        commands.entity(e).insert(SpatialListener::new(0.2));
    }
}

type EmitterQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static SoundEmitter,
        Option<&'static GlobalTransform>,
        Option<&'static mut Voice>,
        Has<AudioPlayer<PcmClip>>,
    ),
>;

/// One emitter's level for this frame: entity, gain, distance (when it has a position) and
/// position.
type EmitterLevel = (Entity, f32, Option<f32>, Option<Vec3>);

/// The level of every emitter: its gain after mute, distance curve and bus volume.
fn emitter_levels(
    settings: &AudioSettings,
    listener_pos: Option<Vec3>,
    emitters: &EmitterQuery,
) -> Vec<EmitterLevel> {
    emitters
        .iter()
        .map(|(entity, e, tf, _, _)| {
            let (distance, spatial_gain) = match (&e.spatial, listener_pos, tf) {
                (Some(curve), Some(l), Some(t)) => {
                    let d = l.distance(t.translation());
                    (Some(d), curve.gain(d))
                }
                // A spatial source with no listener or no position cannot be heard.
                (Some(_), _, _) => (None, 0.0),
                (None, _, _) => (None, 1.0),
            };
            let gain = if e.muted {
                0.0
            } else {
                e.gain * spatial_gain * settings.bus_gain(e.bus)
            };
            (entity, gain, distance, tf.map(GlobalTransform::translation))
        })
        .collect()
}

/// Gives a new voice to an emitter: the player with the clip, the loop mode, volume, speed and the
/// spatial setup.
fn start_voice(commands: &mut Commands, entity: Entity, e: &SoundEmitter, gain: f32) {
    commands.entity(entity).insert((
        AudioPlayer::<PcmClip>(e.clip.clone()),
        PlaybackSettings {
            mode: if e.looping {
                PlaybackMode::Loop
            } else {
                PlaybackMode::Remove
            },
            volume: Volume::Linear(gain),
            speed: e.speed,
            spatial: e.spatial.is_some(),
            // Bevy's own distance fall-off (1/d²) is kept out of the way: positions
            // are scaled into the unit range, our curve does the fading.
            spatial_scale: e.spatial.map(|d| SpatialScale::new(1.0 / d.max.max(1.0))),
            ..PlaybackSettings::ONCE
        },
        Voice { gain },
    ));
}

/// Takes the voice away from an emitter that lost its slot or fell silent; a one-shot is dropped.
fn stop_voice(commands: &mut Commands, entity: Entity, e: &SoundEmitter) {
    commands.entity(entity).remove::<(
        AudioPlayer<PcmClip>,
        PlaybackSettings,
        AudioSink,
        SpatialAudioSink,
        Voice,
    )>();
    if e.despawn_when_done && !e.looping {
        commands.entity(entity).despawn();
    }
}

/// Applies the computed level and the emitter's speed to the sinks of the running voices.
pub fn sync_voice_levels(
    mut voices: Query<(
        &Voice,
        &SoundEmitter,
        Option<&mut AudioSink>,
        Option<&mut SpatialAudioSink>,
    )>,
    mut stats: ResMut<AudioStats>,
) {
    let mut playing = 0;
    for (voice, emitter, plain, spatial) in &mut voices {
        let volume = Volume::Linear(voice.gain);
        if let Some(mut s) = plain {
            s.set_volume(volume);
            s.set_speed(emitter.speed);
            playing += 1;
        }
        if let Some(mut s) = spatial {
            s.set_volume(volume);
            s.set_speed(emitter.speed);
            playing += 1;
        }
    }
    stats.playing = playing;
}
