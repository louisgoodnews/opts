//! The CI rules of `docs/src/roadmap/phase-9/expandability.md` section 7, as functions over a
//! running UI so the library, the game screens and any later screen test the same things.
//!
//! Each function returns the violations as readable lines; an empty list passes. Sizes come from
//! the computed layout when it has run and from the declared styles otherwise (a headless test has
//! no window): that is a floor, the screenshots remain the proof of the real layout.

use crate::actions::UiAction;
use crate::interaction::activation::Activated;
use crate::interaction::state::Disabled;
use crate::not_yet::NotAvailableYet;
use crate::testing::{act, focused};
use crate::theme::contrast::{composite_over, contrast_ratio};
use crate::ui_id::UiId;
use crate::ui_tree::{role_of, snapshot, UiRole};
use bevy::prelude::*;
use size::is_target;

mod colours;
mod contrast;
mod controls;
mod overflow;
mod size;
mod traversal;

pub use colours::hardcoded_colours;
pub use contrast::text_contrast;
pub use controls::{disabled_without_reason, missing_accessibility_nodes, unnamed_controls};
pub use overflow::text_overflow;
pub use size::{effective_size, small_targets, small_text};

pub use traversal::{
    directional_traversal, keyboard_traversal, ActivationLog, DirectionalTraversal, Traversal,
};

/// Smallest interactive target (logical pixels).
pub const MIN_TARGET: f32 = 32.0;
/// Smallest text size (logical pixels).
pub const MIN_TEXT_PX: f32 = 12.0;

/// Result of [`text_contrast`].
#[derive(Debug, Default)]
pub struct ContrastReport {
    /// Text and border pairs that fall below the threshold.
    pub violations: Vec<String>,
    /// Pairs measured.
    pub checked: usize,
    /// Texts drawn over an image: not measurable from the tree (the scrim rule and the screenshots
    /// cover them).
    pub over_image: usize,
}

/// All roles an element may report (a guard that [`role_of`] stays total for the fixtures).
pub fn roles_of_all(world: &mut World) -> Vec<UiRole> {
    let mut q = world.query::<EntityRef>();
    q.iter(world).filter_map(|e| role_of(&e)).collect()
}
