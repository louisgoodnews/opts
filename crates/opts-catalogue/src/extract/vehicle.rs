//! Classifying a vehicle file: its facts, whether it is playable, a trailer or AI-only.

use super::*;

mod analysis;
mod names;
mod detect;

pub(super) use analysis::*;
pub(super) use names::*;
pub(super) use detect::*;









