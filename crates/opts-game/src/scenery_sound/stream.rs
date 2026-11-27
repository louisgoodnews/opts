//! Streams the scenery voices: looping emitters by distance, four times a second.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use opts_bevy::scenery_runtime::{HostValues, SceneryClock};
use opts_bevy::MainCamera;

use crate::audio_content::ClipLoader;
use crate::sound_logic::{conditions_hold, viewpoint_allows, Playback};

use super::loops::{edge_step, loop_step, VoiceQuery};
use super::values::{camera_in_cabin, host_variable};
use super::SceneryAudioStats;

/// The clip loader and the counters of the scenery voices.
#[derive(SystemParam)]
pub struct SceneryVoiceState<'w> {
    loader: ResMut<'w, ClipLoader>,
    stats: ResMut<'w, SceneryAudioStats>,
}

/// Starts and stops the looping emitters of continuous voices by distance, four times a second,
/// gates them by their conditions and plays the `[noloop]` voices whose conditions just turned
/// true.
pub fn stream_scenery_voices(
    time: Res<Time>,
    clock: Res<SceneryClock>,
    listener: Query<&GlobalTransform, With<MainCamera>>,
    camera: Option<Res<opts_bevy::CameraState>>,
    mut voices: VoiceQuery,
    state: SceneryVoiceState,
    mut since: Local<f32>,
    mut commands: Commands,
) {
    let SceneryVoiceState {
        mut loader,
        mut stats,
    } = state;
    *since += time.delta_secs();
    if *since < 0.25 {
        return;
    }
    *since = 0.0;
    let Some(listener) = listener.iter().next().map(GlobalTransform::translation) else {
        return;
    };
    let host = HostValues::of(&clock);
    let read = |n: &str| host_variable(n, &host).unwrap_or(0.0);
    let inside = camera_in_cabin(camera.as_ref().map(|c| c.mode));
    let (mut total, mut playing, mut unsupported, mut unresolved) = (0, 0, 0, 0);
    for (entity, mut voice, tf, emitter) in &mut voices {
        total += 1;
        if voice.playback == Playback::Unsupported {
            unsupported += 1;
            continue;
        }
        let Some(path) = voice.file.clone() else {
            unresolved += 1;
            continue;
        };
        // A voice the camera's viewpoint excludes is silent, as if its conditions did not hold.
        let active = conditions_hold(&voice.def.conditions, read).unwrap_or(false)
            && viewpoint_allows(voice.def.viewpoint, inside);
        let d = listener.distance(tf.translation());
        match voice.playback {
            Playback::Loop => {
                playing += loop_step(
                    &mut commands,
                    &mut loader,
                    (entity, &voice, emitter),
                    (&path, d, active),
                    read,
                );
            }
            Playback::OnEdge => {
                edge_step(
                    &mut commands,
                    (&mut loader, &mut stats),
                    (entity, &mut voice, &path),
                    (d, active),
                    read,
                );
            }
            Playback::OnTrigger | Playback::Unsupported => {}
        }
    }
    stats.voices = total;
    stats.playing = playing;
    stats.unsupported = unsupported;
    stats.unresolved = unresolved;
}
