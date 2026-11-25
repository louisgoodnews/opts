//! The colour *look* of the 3D view: tonemapper, post-saturation, contrast and ambient scale.
//!
//! The types [`Look`] (the user setting: settings screen, `--look`, control protocol `render.look`)
//! and [`LookParams`] (what a look does to the camera and the sky) live in the `opts-game-state`
//! crate and are re-exported here. The [`ActiveLook`] resource holds the look in effect;
//! [`LookPlugin`] applies its parameters live to the [`MainCamera`](opts_bevy::MainCamera)
//! ([`apply_look_to_cameras`]) and the sky system reads [`LookParams::ambient_scale`].
//!
//! Spec: `docs/src/architecture/texture-quality.md` §3.2 and `docs/src/roadmap/phase-8/M-0060.md`.

mod apply;
mod grading;

pub use apply::{apply_look_to_cameras, apply_startup_render_options, LookApplied, LookPlugin};
#[cfg(test)]
pub(crate) use grading::somewhat_boring;
pub use grading::{daylight, effective_ambient_scale, grading_for, hdr_clear_color};
pub use opts_game_state::look::{Look, LookParams};

use bevy::prelude::*;
#[cfg(test)]
use bevy::render::view::Tonemapping;

/// The look in effect. Starts as the setting (or the `--look` override of this run) and changes
/// live through the settings screen and the control protocol.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ActiveLook(pub Look);

#[cfg(test)]
mod tests;
