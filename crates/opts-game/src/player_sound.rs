//! Sounds of the player's vehicle (M-0102): the `[sound]` configuration of the `.bus` becomes
//! [`PlayerVoice`] children of the vehicle, driven by the vehicle's script variables and events.
//!
//! * a voice that is a loop while active (`engine_n` pitch, `velocity` rolling noise, the
//!   compressor, wipers, sirens …) has a looping emitter near the listener, its level from the
//!   `[volcurve]`s, its pitch from the loop's variable, silenced while its `[conditionSingle]`s
//!   do not hold;
//! * a voice that fires once (`[trigger]` on a raised script event, or `[noloop]` with a condition
//!   that turns true) plays a one-shot at its `[3d]` place (doors, switches, kneeling, starter);
//! * `[viewpoint]` 2 plays outside the vehicle only, 5 inside only.
//!
//! The rules of when a definition plays are in [`crate::sound_logic`]. Not yet: trailers and the
//! articulated bus' second unit, announcements (`(T.F)` payloads), the `Snd_*` mixer inputs,
//! force feedback, the horn and other definitions with a condition mode that is not understood
//! (counted in [`PlayerAudioStats::unsupported`]).

mod attach;
mod listener;
mod steps;
mod stream;
mod triggers;
mod types;

pub use attach::{attach, attach_defs, attach_unit};
pub use listener::{listener_in_cabin, listener_inside, PlayerListener};
pub use stream::{stream_player_voices, PlayerVoiceState};
pub use triggers::{
    fire_player_triggers, preload_one_shots, smooth_player_voices, PlayerAudioPlugin,
};
pub use types::{PlayerAudioStats, PlayerSoundInfo, PlayerVoice};

#[cfg(test)]
mod tests;
