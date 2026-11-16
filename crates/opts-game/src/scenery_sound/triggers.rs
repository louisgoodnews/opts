//! Plays the scenery one-shots whose `[trigger]` the objects' scripts raised.

use bevy::prelude::*;
use opts_bevy::audio::{SoundBus, SoundEmitter};
use opts_bevy::scenery_runtime::{HostValues, SceneryClock};
use opts_bevy::SceneryScripts;

use crate::audio_content::{ClipLoader, ClipState};
use crate::sound_logic::{conditions_hold, Playback};

use super::values::host_variable;
use super::{SceneryAudioStats, SceneryVoice};

/// Plays the one-shots whose `[trigger]` the objects' scripts raised this frame.
pub fn fire_scenery_triggers(
    clock: Res<SceneryClock>,
    mut objects: Query<(&mut SceneryScripts, &Children)>,
    voices: Query<(&SceneryVoice, &GlobalTransform)>,
    mut loader: ResMut<ClipLoader>,
    mut stats: ResMut<SceneryAudioStats>,
    mut commands: Commands,
) {
    let host = HostValues::of(&clock);
    for (mut scripts, children) in &mut objects {
        // No change flag for the thousands of objects that raised nothing.
        let raised = {
            let s = scripts.bypass_change_detection();
            if !s.has_raised() {
                continue;
            }
            s.take_raised()
        };
        for name in raised {
            for child in children.iter() {
                let Ok((voice, tf)) = voices.get(child) else {
                    continue;
                };
                if voice.playback != Playback::OnTrigger
                    || !voice
                        .def
                        .triggers
                        .iter()
                        .any(|t| t.trim().eq_ignore_ascii_case(&name))
                    || conditions_hold(&voice.def.conditions, |n| {
                        host_variable(n, &host).unwrap_or(0.0)
                    }) != Some(true)
                {
                    continue;
                }
                let Some(path) = voice.file.as_deref() else {
                    continue;
                };
                match loader.state(path) {
                    ClipState::Ready(clip) => {
                        stats.one_shots += 1;
                        commands.spawn((
                            SoundEmitter::new(clip, SoundBus::Ambient)
                                .spatial(voice.distance())
                                .with_speed(voice.def.pitch),
                            Transform::from_translation(tf.translation()),
                            Name::new("scenery one-shot"),
                        ));
                    }
                    // The first ask only starts the decode; the next trigger is heard.
                    ClipState::Loading => stats.cold_misses += 1,
                    ClipState::Failed => {}
                }
            }
        }
    }
}
