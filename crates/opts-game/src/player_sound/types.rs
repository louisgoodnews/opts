//! The types of the player's sounds: the voice component, the vehicle's sound summary and the
//! statistics `audio.state` reads.

use std::path::PathBuf;
use std::sync::Arc;

use bevy::prelude::*;
use opts_bevy::audio::Distance;
use opts_core::formats::sound::SoundDef;

use crate::sound_logic::Playback;

/// One sound definition of the player's vehicle, a child at its `[3d]` position.
#[derive(Component, Debug, Clone)]
pub struct PlayerVoice {
    /// The definition.
    pub def: Arc<SoundDef>,
    /// The sound file.
    pub file: PathBuf,
    /// What it does when active.
    pub playback: Playback,
    /// The conditions held at the last look (edge detection of `[noloop]` voices).
    pub was_active: bool,
    /// The vehicle whose script variables and events drive this voice when it hangs on another
    /// unit of it (a trailer shares the head's scripts, `[scriptshare]`); `None`: its parent.
    pub owner: Option<Entity>,
    /// Where the emitter's gain and speed are heading (set twenty times a second, followed every
    /// frame by [`smooth_player_voices`]): a change of the engine speed is a glide, not steps.
    pub goal: Option<(f32, f32)>,
    /// Seconds the voice has been active without a break: the volume curves with the variable
    /// `-1` ([`crate::sound_logic::AGE_VARIABLE`]) fade the voice in over it.
    pub active_s: f32,
}

impl PlayerVoice {
    /// The fade with distance: full inside the `[3d]` range, silent beyond six times it (at least
    /// 40 m more). Cabin sounds have ranges of a metre or two and are heard around the vehicle.
    #[must_use]
    pub fn distance(&self) -> Distance {
        let r = self.def.range.max(0.5);
        Distance::new(r, (r * 6.0).max(r + 40.0))
    }
}

/// What the definitions of a vehicle's configuration came to (on the vehicle root).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct PlayerSoundInfo {
    /// Definitions in the configuration.
    pub total: usize,
    /// Left out: a condition mode that is not understood, or `[noloop]` alone.
    pub unsupported: usize,
    /// Left out: the sound file was not found.
    pub unresolved: usize,
}

/// What the player's sounds did, for `audio.state`.
#[derive(Resource, Debug, Clone, Default)]
pub struct PlayerAudioStats {
    /// Voice entities.
    pub voices: usize,
    /// Loops with an emitter now (the listener is within their range).
    pub loops: usize,
    /// One-shots started.
    pub one_shots: u64,
    /// Triggers that found their file still decoding.
    pub cold_misses: u64,
    /// Definitions left out for an unsupported condition mode.
    pub unsupported: usize,
    /// Definitions whose file was not found.
    pub unresolved: usize,
    /// The listener is inside the vehicle.
    pub inside: bool,
    /// The last script events the sound layer saw and how many one-shot voices each started (a
    /// name with 0 has no voice in the vehicle's configuration, or none whose conditions held).
    pub events: std::collections::VecDeque<(String, usize)>,
}
