//! Focus candidates, visual order, regions and spatial navigation.
//!
//! [`FocusGraph`] answers "which controls can take focus right now, in which order": a control
//! qualifies when it is [`Focusable`], visible (no `Display::None`, no `Visibility::Hidden` on it
//! or an ancestor), enabled or disabled *with a reason* (so the reason can be read), and inside
//! the topmost modal while one is open. Tab order is the order of the layout tree (a depth-first
//! walk; `tab_index` is not consulted); arrow keys use the real geometry.

use crate::components::modal::ModalContent;
use crate::interaction::focus::{FocusRegion, Focusable, PointerOnly};
use crate::interaction::state::{Disabled, DisabledReason};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

mod directional;
mod graph;
mod linear;

pub use directional::{move_focus, pick_directional, pick_directional_weighted};
pub use linear::{next_linear, next_region};

/// One control that can take focus.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    /// The control.
    pub entity: Entity,
    /// Layout-tree path used as the order key.
    pub order: Vec<u32>,
    /// Screen rectangle in physical pixels, when the layout has run.
    pub rect: Option<Rect>,
    /// Nearest ancestor-or-self carrying [`FocusRegion`].
    pub region: Option<Entity>,
}

/// Read access to everything the focus order is computed from.
#[derive(SystemParam)]
pub struct FocusGraph<'w, 's> {
    focusables: Query<'w, 's, (Entity, Has<Disabled>, Has<DisabledReason>), With<Focusable>>,
    nodes: Query<'w, 's, (&'static Node, Option<&'static Visibility>)>,
    parents: Query<'w, 's, &'static ChildOf>,
    children: Query<'w, 's, &'static Children>,
    geometry: Query<'w, 's, (&'static ComputedNode, &'static UiGlobalTransform)>,
    regions: Query<'w, 's, (), With<FocusRegion>>,
    modals: Query<'w, 's, Entity, With<ModalContent>>,
    pointer_only: Query<'w, 's, (), With<PointerOnly>>,
}
#[cfg(test)]
mod tests;
