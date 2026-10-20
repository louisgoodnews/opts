//! The sound engine (M-0027): decoded clips ([`PcmClip`]), sources the game places
//! ([`SoundEmitter`]), the gain curves ([`curves`]) and the systems that decide what is heard
//! ([`engine`]). Playback goes through Bevy's audio output; the engine adds what a simulator
//! needs on top: a voice budget (the loudest sources win), distance fading with a hard range so
//! a far source costs nothing, mix buses with volume settings, and one-shots that clean up after
//! themselves. Design and numbers: `docs/src/architecture/audio-system.md`.
//!
//! The engine knows no file format: the game decodes OMSI's WAV files to [`PcmClip`]s and adds
//! emitters; later milestones place them (scenery objects and splines, AI traffic, the player's
//! vehicle).

pub mod clip;
pub mod curves;
pub mod emitter;
pub mod engine;

use bevy::audio::AddAudioSource;
use bevy::prelude::*;

pub use clip::PcmClip;
pub use curves::{db_to_gain, percent_to_gain, semitones_to_speed, Distance};
pub use emitter::{SoundBus, SoundEmitter, Voice};
pub use engine::{AudioSettings, AudioStats, EmitterReport, AUDIBLE_MIN};

/// System set of the engine; runs in `PostUpdate` after transform propagation, so positions are
/// this frame's.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct AudioEngineSet;

/// Registers the engine. Add it after Bevy's `AudioPlugin` (part of `DefaultPlugins`); without
/// one the engine still selects voices and keeps its statistics, nothing is played.
#[derive(Default)]
pub struct OptsAudioPlugin;

impl Plugin for OptsAudioPlugin {
    fn build(&self, app: &mut App) {
        if app.is_plugin_added::<bevy::audio::AudioPlugin>() {
            app.add_audio_source::<PcmClip>();
        } else {
            app.init_asset::<PcmClip>();
        }
        app.init_resource::<AudioSettings>()
            .init_resource::<AudioStats>()
            .add_systems(
                PostUpdate,
                (
                    engine::ensure_listener,
                    engine::select_voices,
                    engine::sync_voice_levels,
                )
                    .chain()
                    .in_set(AudioEngineSet)
                    .after(bevy::transform::TransformSystems::Propagate),
            );
    }
}

#[cfg(test)]
mod tests;
