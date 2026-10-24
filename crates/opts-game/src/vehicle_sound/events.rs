//! The events the traffic model raises for an AI vehicle (the horn, `ev_AI_Horn`) and the
//! one-shots of the AI sound configuration that answer them.

use bevy::prelude::*;
use opts_bevy::audio::{SoundBus, SoundEmitter};
use opts_bevy::{AiVariables, MainCamera};
use std::path::PathBuf;
use std::sync::Arc;

use super::AiAudioStats;
use crate::audio_content::{ClipLoader, ClipState};
use opts_core::formats::sound::SoundDef;

/// An event the traffic model raises for an AI vehicle (`ev_AI_Horn`): it plays the one-shots of
/// the configuration that name it.
#[derive(Component, Debug, Clone, Default)]
pub struct AiEvents(pub Vec<String>);

impl AiEvents {
    /// Raises event `name`.
    pub fn raise(&mut self, name: &str) {
        if self.0.len() < 8 {
            self.0.push(name.to_owned());
        }
    }
}

/// A one-shot of an AI vehicle on a host event, a child at its `[3d]` position.
#[derive(Component, Debug, Clone)]
pub struct AiEventVoice {
    /// The definition.
    pub def: Arc<SoundDef>,
    /// The sound file.
    pub file: PathBuf,
}

/// Plays the one-shots whose event the traffic model raised for an AI vehicle.
pub fn fire_ai_events(
    listener: Query<&GlobalTransform, With<MainCamera>>,
    mut vehicles: Query<(Entity, &mut AiEvents, &AiVariables)>,
    voices: Query<(&AiEventVoice, &GlobalTransform, &ChildOf)>,
    mut loader: ResMut<ClipLoader>,
    mut stats: ResMut<AiAudioStats>,
    mut commands: Commands,
) {
    let listener = listener.iter().next().map(GlobalTransform::translation);
    for (vehicle, mut events, vars) in &mut vehicles {
        if events.0.is_empty() {
            continue;
        }
        let raised = std::mem::take(&mut events.0);
        for name in raised {
            for (voice, tf, parent) in &voices {
                if parent.parent() != vehicle
                    || !voice
                        .def
                        .triggers
                        .iter()
                        .any(|t| t.trim().eq_ignore_ascii_case(&name))
                    || crate::sound_logic::conditions_hold(&voice.def.conditions, |n| vars.get(n))
                        != Some(true)
                {
                    continue;
                }
                let r = voice.def.range.max(0.5);
                let curve = opts_bevy::audio::Distance::new(r, (r * 6.0).max(r + 40.0));
                // Far horns are not heard: do not decode for them.
                if listener.is_some_and(|l| l.distance(tf.translation()) > curve.max) {
                    continue;
                }
                if let ClipState::Ready(clip) = loader.state(&voice.file) {
                    stats.events += 1;
                    let level = crate::sound_logic::level(&voice.def, |n| vars.get(n));
                    commands.entity(vehicle).with_children(|p| {
                        p.spawn((
                            SoundEmitter::new(clip, SoundBus::Vehicles)
                                .spatial(curve)
                                .with_speed(voice.def.pitch.clamp(0.3, 3.5))
                                .with_gain(level.max(0.05)),
                            Transform::IDENTITY,
                            Name::new("ai one-shot"),
                        ));
                    });
                }
            }
        }
    }
}
