//! Animation pivots, articulation runs and skin binding.

use super::*;

mod binding;
mod pivots;
mod runs;
mod trailer;

pub(super) use binding::{bind_skins, joint_slots, SkinBinding};
#[cfg(test)]
pub(super) use pivots::referenced_idents;
pub(super) use pivots::{anim_pivots, forms_cycle, is_bone_stem, mesh_pivot, pivot_of};
pub(super) use runs::{articulation_runs, joints_in_frame, pick_run};
pub(super) use trailer::{is_articulated_bellow, trailer_link, TrailerLink};
