//! The sound layer as the world load sees it (ISS-064): the data the load hands to the sound
//! modules of `opts-game` (`vehicle_sound`, `player_sound`, `scenery_sound`) and the hook through
//! which it calls them. The sound modules stay in `opts-game`; this crate does not depend on them.

use std::path::PathBuf;
use std::sync::Arc;

use bevy::prelude::*;
use opts_core::formats::sound::SoundDef;

/// The definitions of an AI vehicle type that can be played, with their sound files.
#[derive(Debug, Clone, Default)]
pub struct AiSoundSet {
    /// The loops the traffic model can drive (at most 16) with their files:
    /// continuous, their conditions and variables all stated by the model.
    pub voices: Vec<(Arc<SoundDef>, PathBuf)>,
    /// One-shots the traffic model raises itself (the horn, `ev_ai_horn`).
    pub events: Vec<(Arc<SoundDef>, PathBuf)>,
    /// Definitions that need the vehicle's own scripts (door triggers and door variables,
    /// anything else the model does not state): played only by a vehicle that runs scripts, a
    /// scheduled bus (at most 48).
    pub scripted: Vec<(Arc<SoundDef>, PathBuf)>,
    /// Definitions of the configuration that were left out (a condition mode that is not
    /// understood, interior-only sounds, no file, over a cap).
    pub skipped: usize,
}

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
