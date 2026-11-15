//! When a sound definition plays (`docs/src/formats/text/vehicles/sound_cfg.md`): shared by the
//! scenery, AI and player-vehicle layers so that all three read a configuration the same way.
//!
//! A definition is **active** when its `[conditionSingle]`s hold (all of them), or always when it
//! has none. What activation does depends on its keywords:
//!
//! | keywords | plays |
//! | :--- | :--- |
//! | `[loopsound]`, or `[sound]`, no trigger, no `[noloop]` | as a loop while active |
//! | `[noloop]` and conditions, no trigger | once, when it becomes active |
//! | `[trigger]` | once per raised event (while active, when it has conditions) |
//!
//! The comparison of a `[conditionSingle] variable value mode` is read from usage (13,270
//! conditions in the installed vehicles; ❓ Q-SND-003): mode 0 is "differs" (the horn's `cockpit_hupe_volume 3 0`, a volume that is never 3, so the loop is always on and its volume curve decides; `antrieb_getr_aktugang 0 0`: a gear is engaged), mode 1 is "equals" (`engine_on 0`,
//! `antrieb_wendesatz 1`, `wiperrunning 1`: 9,659), mode 2 is "below" (`velocity 1`,
//! `throttle 0.1`, `brake 0.02`: stopped, off the pedal), mode 3 is "above" (`engine_n 300`,
//! `engine_n 200`, `velocity 3`: engine running, driving). Modes 4 ("at least": `antrieb_getr_aktugang 2`) and 5
//! ("at most": `antrieb_retarderstufe 2, 3, 4`) complete the set 0 to 5 (differs, equals, below,
//! above, at least, at most); -1 (2 uses) is not understood and makes a definition
//! [`Playback::Unsupported`].

use opts_core::formats::sound::{Condition, SoundDef, SoundKind};

mod levels;
mod playback;
mod speed;
mod viewpoint;
pub use levels::{compare, conditions_hold, level, AGE_VARIABLE};
pub use playback::{playback_of, Playback};
pub use speed::{loop_voice, speed, MIN_SPEED};
pub use viewpoint::viewpoint_allows;

#[cfg(test)]
mod tests;
