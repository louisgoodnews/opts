//! What the sound engine reports: the emitter list and the counts behind `audio.state`.

use super::*;

/// One emitter as the statistics list it.
#[derive(Debug, Clone, PartialEq)]
pub struct EmitterReport {
    /// The entity.
    pub entity: Entity,
    /// Computed level (source × distance × bus × master).
    pub gain: f32,
    /// Metres from the listener (`None` for a source that is not spatial).
    pub distance: Option<f32>,
    /// Whether it has a playing voice.
    pub voice: bool,
    /// The bus.
    pub bus: SoundBus,
    /// Where it is (Bevy world metres), when it has a position.
    pub position: Option<Vec3>,
}

/// What the engine did last frame.
#[derive(Resource, Debug, Clone, Default)]
pub struct AudioStats {
    /// All emitters.
    pub emitters: usize,
    /// Emitters loud enough to be heard.
    pub audible: usize,
    /// Emitters with a voice.
    pub voices: usize,
    /// Audible emitters left without a voice by the budget.
    pub virtualized: usize,
    /// Voices whose sink exists (the sound card accepted them).
    pub playing: usize,
    /// The listener's position, if there is one.
    pub listener: Option<Vec3>,
    /// The loudest emitters (at most 16).
    pub top: Vec<EmitterReport>,
}
