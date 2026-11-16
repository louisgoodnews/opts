//! Sounds of AI traffic (M-0140): the vehicle's AI sound configuration (`[sound_ai]` of a `.bus`;
//! for cars the shared `Sounds/AI_Cars` file their `[sound]` names) becomes [`AiVoice`] children of
//! the vehicle, and a system plays the continuous ones (engine, rolling noise, wheels) near the
//! listener with levels and pitch from the vehicle's [`AiVariables`].
//!
//! AI vehicles run without their scripts, so only the variables the traffic model can state are
//! offered (`velocity`, `throttle`, `brake`, `engine_n` from a gear model ❓, the wheel speeds,
//! `streetcond`, …; see [`supported_variable`]). A definition that reads any other variable, needs
//! a trigger or a condition, is left out when the vehicle type loads and counted in
//! [`AiSoundSet::skipped`]: of the 45,043 definitions in the 499 AI configurations of the
//! installation, 6,319 qualify (median 8 per configuration).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use bevy::prelude::*;
use opts_bevy::audio::{SoundBus, SoundEmitter};
use opts_bevy::{omsi_to_bevy_pos, AiVariables, MainCamera};
use opts_core::formats::sound::{SoundConfig, SoundDef, SoundKind};
use opts_core::formats::vehicles::BusManifest;

use crate::audio_content::{ClipLoader, ClipState};
use crate::in_game::omsi_world::WorldLoadJob;
use crate::scenery_sound::SceneryVoice;

/// Most continuous voices one AI vehicle carries.
pub const MAX_AI_VOICES: usize = 16;

/// The definitions of an AI vehicle type that can be played, with their sound files.
#[derive(Debug, Clone, Default)]
pub struct AiSoundSet {
    /// The loops the traffic model can drive (at most [`MAX_AI_VOICES`]) with their files:
    /// continuous, their conditions and variables all stated by the model.
    pub voices: Vec<(Arc<SoundDef>, PathBuf)>,
    /// One-shots the traffic model raises itself (the horn, [`HOST_EVENTS`]).
    pub events: Vec<(Arc<SoundDef>, PathBuf)>,
    /// Definitions that need the vehicle's own scripts (door triggers and door variables,
    /// anything else the model does not state): played only by a vehicle that runs scripts, a
    /// scheduled bus (at most [`MAX_SCRIPTED_VOICES`]).
    pub scripted: Vec<(Arc<SoundDef>, PathBuf)>,
    /// Definitions of the configuration that were left out (a condition mode that is not
    /// understood, interior-only sounds, no file, over a cap).
    pub skipped: usize,
}

/// Most script-driven voices a scheduled AI vehicle carries.
pub const MAX_SCRIPTED_VOICES: usize = 48;
/// The events the traffic model raises for an AI vehicle (lower case): `ev_AI_Horn`.
pub const HOST_EVENTS: [&str; 1] = ["ev_ai_horn"];

mod select;
pub use select::{ai_sound_set, choose, host_event, is_ai_name, playable, supported_variable};


/// One continuous sound of an AI vehicle, a child at its `[3d]` position.
#[derive(Component, Debug, Clone)]
pub struct AiVoice {
    /// The definition.
    pub def: Arc<SoundDef>,
    /// The sound file.
    pub file: PathBuf,
    /// The head of the vehicle whose variables drive this voice when it hangs on a following
    /// unit (the second unit of an articulated bus has its own configuration at its own places
    /// and the head's variables); `None`: its parent.
    pub owner: Option<Entity>,
}

mod events;
pub use events::{fire_ai_events, AiEventVoice, AiEvents};

/// What the AI sounds did, for `audio.state`.
#[derive(Resource, Debug, Clone, Default)]
pub struct AiAudioStats {
    /// Voice entities of all AI vehicles.
    pub voices: usize,
    /// Voices with an emitter now.
    pub playing: usize,
    /// One-shots started by events the traffic model raised (the horn).
    pub events: u64,
}

/// Adds the voices of a vehicle type under `root`.
pub fn attach_ai(commands: &mut Commands, root: Entity, set: Option<&Arc<AiSoundSet>>) {
    let Some(set) = set else { return };
    attach_ai_unit(commands, root, None, set);
    commands
        .entity(root)
        .insert(AiEvents::default())
        .with_children(|p| {
            for (def, file) in &set.events {
                let at = def.position.unwrap_or([0.0; 3]);
                p.spawn((
                    AiEventVoice {
                        def: def.clone(),
                        file: file.clone(),
                    },
                    Transform::from_translation(omsi_to_bevy_pos(Vec3::new(at[0], at[1], at[2]))),
                    Name::new("AiEventVoice"),
                ));
            }
        });
}

/// Adds the loops of one unit's sound set under `unit`: for a following unit of an articulated
/// bus `owner` is the head, whose variables drive them.
pub fn attach_ai_unit(
    commands: &mut Commands,
    unit: Entity,
    owner: Option<Entity>,
    set: &Arc<AiSoundSet>,
) {
    commands.entity(unit).with_children(|p| {
        for (def, file) in &set.voices {
            let at = def.position.unwrap_or([0.0; 3]);
            p.spawn((
                AiVoice {
                    def: def.clone(),
                    file: file.clone(),
                    owner,
                },
                Transform::from_translation(omsi_to_bevy_pos(Vec3::new(at[0], at[1], at[2]))),
                Name::new("AiVoice"),
            ));
        }
    });
}

mod stream;
pub use stream::{
    ai_engine_rpm, ai_gear, level_of, publish_sound_variables, speed_of, stream_ai_voices,
    AiAudioPlugin,
};

#[cfg(test)]
mod tests;
