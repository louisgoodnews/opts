//! Classifying a vehicle file: its facts, whether it is playable, a trailer or AI-only.

use super::*;

mod analysis;
mod detect;
mod names;

pub(super) use analysis::*;
pub(super) use detect::*;
pub(super) use names::*;
