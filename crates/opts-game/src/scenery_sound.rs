//! Sounds of scenery objects (M-0139): the `[sound]` configuration of a `.sco` becomes
//! [`SceneryVoice`] children of the placed object, and three systems turn them into sound:
//!
//! * **continuous voices** (a `[sound]` without `[trigger]`, `[conditionSingle]` or `[noloop]`:
//!   hums, ambience, birdsong; ❓ inferred, see `docs/src/formats/text/vehicles/sound_cfg.md`) get
//!   a looping [`opts_bevy::audio::SoundEmitter`] while the listener is within their range, with a level from their
//!   `[volcurve]`s over the session clock (`Time`, `DayOfYear`, `SunAlt`);
//! * **triggered voices** (`[trigger] name`) play a one-shot when the object's script raises that
//!   event (`(T.L.name)`), at the voice's place;
//! * clips are decoded on worker threads ([`ClipLoader`]) the first time a voice comes near.
//!
//! `[viewpoint]` gates a voice by the camera mode (cabin views for 2 and 6, outside for 5 and 1;
//! ❓ Q-SND-004, see `values::camera_in_cabin`).
//!
//! Not yet: `[conditionSingle]` (counted in [`SceneryAudioStats::unsupported`]), `[onlyone]`,
//! `[checkloading]`, and the objects' own script variables in `[volcurve]`.

use std::path::PathBuf;
use std::sync::Arc;

use bevy::prelude::*;
use opts_bevy::audio::Distance;
use opts_core::formats::sound::SoundDef;

use crate::audio_content::{poll_clip_loader, ClipLoader};
use crate::sound_logic::Playback;

mod attach;
mod loops;
mod stream;
mod triggers;
mod values;

pub use attach::attach;
pub use stream::{stream_scenery_voices, SceneryVoiceState};
pub use triggers::fire_scenery_triggers;
pub use values::{host_variable, level_of};

/// One sound definition of a placed object, a child at its `[3d]` position.
#[derive(Component, Debug, Clone)]
pub struct SceneryVoice {
    /// The definition.
    pub def: Arc<SoundDef>,
    /// The sound file, when it was found.
    pub file: Option<PathBuf>,
    /// What it does when active ([`crate::sound_logic::playback_of`]).
    pub playback: Playback,
    /// The conditions held at the last look (edge detection of `[noloop]` voices).
    pub was_active: bool,
}

impl SceneryVoice {
    /// How the voice fades: full level inside the `[3d]` range, silent beyond six times that
    /// (at least 40 m more). OMSI's own maximum distance is not known (Q-AU01).
    #[must_use]
    pub fn distance(&self) -> Distance {
        let r = self.def.range.max(0.5);
        Distance::new(r, (r * 6.0).max(r + 40.0))
    }
}

/// What the scenery sounds did, for `audio.state`.
#[derive(Resource, Debug, Clone, Default)]
pub struct SceneryAudioStats {
    /// Voice entities (all definitions of all placed objects).
    pub voices: usize,
    /// Continuous voices with an emitter now (within range, clip ready).
    pub playing: usize,
    /// Triggered one-shots started.
    pub one_shots: u64,
    /// Triggers that found their clip still loading.
    pub cold_misses: u64,
    /// Definitions that wait for a `[conditionSingle]` (not supported yet).
    pub unsupported: usize,
    /// Definitions whose sound file was not found.
    pub unresolved: usize,
    /// The events the objects raised, oldest first, each with how many voices took it (0: no
    /// sound is mapped to it). The last 64 are kept.
    pub events: std::collections::VecDeque<(String, usize)>,
}

/// Registers the scenery sound systems.
#[derive(Default)]
pub struct SceneryAudioPlugin;

impl Plugin for SceneryAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ClipLoader>()
            .init_resource::<SceneryAudioStats>()
            .add_systems(
                Update,
                (
                    poll_clip_loader,
                    stream_scenery_voices,
                    fire_scenery_triggers,
                )
                    .chain()
                    .after(opts_bevy::scenery_runtime::SceneryRuntimeSet),
            );
    }
}

#[cfg(test)]
mod tests;
