//! What a game system adds to an entity to make it heard: a [`SoundEmitter`].

use bevy::prelude::*;

use super::clip::PcmClip;
use super::curves::Distance;

/// The mix groups with their own volume setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SoundBus {
    /// One-off effects (switches, doors, horns).
    Effects,
    /// The world's own sounds (scenery objects, splines, weather).
    Ambient,
    /// Engines, tyres and brakes of vehicles.
    Vehicles,
    /// The interface.
    Ui,
}

impl SoundBus {
    /// The settings key of the bus' volume.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::Effects => "audio.effects",
            Self::Ambient => "audio.ambience",
            Self::Vehicles => "audio.engine",
            Self::Ui => "audio.interface",
        }
    }
}

/// A sound source. The engine decides each frame whether it is heard (distance, volume settings,
/// the voice budget) and starts or stops the actual playback; the game only keeps this component
/// true to what the thing makes: its `gain`, `speed` and position.
#[derive(Component, Debug, Clone)]
pub struct SoundEmitter {
    /// What it plays.
    pub clip: Handle<PcmClip>,
    /// Which mix group it belongs to.
    pub bus: SoundBus,
    /// Linear level of the source itself (before distance and settings), 0 and up.
    pub gain: f32,
    /// Playback speed: 1 is the recorded pitch, 2 is an octave up.
    pub speed: f32,
    /// Plays again and again until the emitter is removed.
    pub looping: bool,
    /// Fades with distance from the listener and is panned; `None` plays everywhere (the UI,
    /// the player's own cabin sounds).
    pub spatial: Option<Distance>,
    /// Silences the source without removing it.
    pub muted: bool,
    /// Despawns the entity once a one-shot has played (or could not be heard at its start).
    pub despawn_when_done: bool,
}

impl SoundEmitter {
    /// A one-shot at full level, not spatial, that despawns its entity when done.
    #[must_use]
    pub fn new(clip: Handle<PcmClip>, bus: SoundBus) -> Self {
        Self {
            clip,
            bus,
            gain: 1.0,
            speed: 1.0,
            looping: false,
            spatial: None,
            muted: false,
            despawn_when_done: true,
        }
    }

    /// Plays in a loop and stays after it was heard.
    #[must_use]
    pub fn looping(mut self) -> Self {
        self.looping = true;
        self.despawn_when_done = false;
        self
    }

    /// Fades with `distance` and is panned.
    #[must_use]
    pub fn spatial(mut self, distance: Distance) -> Self {
        self.spatial = Some(distance);
        self
    }

    /// Sets the source level.
    #[must_use]
    pub fn with_gain(mut self, gain: f32) -> Self {
        self.gain = gain.max(0.0);
        self
    }

    /// Sets the playback speed.
    #[must_use]
    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed.max(0.01);
        self
    }
}

/// Marks an emitter that currently has a playing voice (the engine adds and removes it).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Voice {
    /// The linear level the sink was last set to.
    pub gain: f32,
}
