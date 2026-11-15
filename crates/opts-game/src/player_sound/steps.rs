//! The per-voice steps of the streaming system: a loop's emitter and gain, an edge's one-shot,
//! and the reading of the vehicle's variables.

use bevy::prelude::*;
use opts_bevy::audio::{SoundBus, SoundEmitter};
use opts_bevy::VehicleScripts;

use crate::audio_content::{ClipLoader, ClipState};
use crate::sound_logic::{level, loop_voice};

use super::triggers::fire;
use super::types::{PlayerAudioStats, PlayerVoice};

/// Reads a vehicle variable by name (case-insensitive).
pub(super) fn variable(scripts: &VehicleScripts, name: &str) -> f32 {
    scripts
        .engine
        .lock()
        .map_or(0.0, |e| e.get(&name.to_lowercase()))
}

/// A looping voice of the player's vehicle: drops its emitter when the listener is far away, sets
/// the gain and speed the emitter follows, and starts the emitter of a loop that begins to sound.
/// Returns whether the voice has an emitter that keeps playing.
pub(super) fn loop_step(
    commands: &mut Commands,
    loader: &mut ClipLoader,
    (entity, voice, emitter): (Entity, &mut PlayerVoice, Option<&SoundEmitter>),
    (d, sounding): (f32, bool),
    read: impl Fn(&str) -> f32 + Copy,
) -> bool {
    let mut counted = false;
    let curve = voice.distance();
    match emitter {
        Some(e) => {
            if d > curve.max * 1.25 {
                commands.entity(entity).remove::<SoundEmitter>();
                return false;
            }
            counted = true;
            let (gain, spd) = if sounding {
                loop_voice(&voice.def, read)
            } else {
                (0.0, e.speed)
            };
            // The emitter follows these every frame (`smooth_player_voices`).
            voice.goal = Some((gain, spd));
        }
        None if d <= curve.max && sounding => {
            let (gain, spd) = loop_voice(&voice.def, read);
            // A loop that is silent now gets its emitter when it sounds.
            if gain > 0.0 {
                if let ClipState::Ready(clip) = loader.state(&voice.file) {
                    commands.entity(entity).insert(
                        SoundEmitter::new(clip, SoundBus::Vehicles)
                            .looping()
                            .spatial(curve)
                            .with_speed(spd)
                            .with_gain(gain),
                    );
                }
            }
        }
        None => {}
    }
    counted
}

/// An edge-triggered voice: plays its one-shot when its conditions just turned true and the
/// listener's view lets it be heard.
pub(super) fn edge_step(
    commands: &mut Commands,
    (loader, stats): (&mut ClipLoader, &mut PlayerAudioStats),
    (entity, voice): (Entity, &mut PlayerVoice),
    (active, audible_here): (bool, bool),
    read: impl Fn(&str) -> f32 + Copy,
) {
    let edge = active && !voice.was_active;
    if voice.was_active != active {
        voice.was_active = active;
    }
    if edge && audible_here {
        let level = level(&voice.def, read);
        fire(commands, loader, stats, entity, voice, level);
    }
}
