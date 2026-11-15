//! Material Design 3 Toplevel / Floating modal window dialog widget.

use bevy::prelude::*;

/// Marker component for an M3 Toplevel floating window root entity.
#[derive(Component, Debug, Clone, Default)]
pub struct M3Toplevel {
    pub title: String,
    pub is_modal: bool,
}

/// Marker component for the window close button.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToplevelCloseButton(pub Entity);

/// Marker component for the inner content body of a toplevel window.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToplevelBody(pub Entity);

mod builder;
mod populate;
mod systems;

pub use builder::ToplevelBuilder;
pub use systems::{ToplevelPlugin, toplevel_close_system};

#[cfg(test)]
mod tests;
