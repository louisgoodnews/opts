//! Interaction state marker components for widgets.

use bevy::prelude::*;

/// Marker component indicating that a widget is currently hovered by a pointer.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Hovered;

/// Marker component indicating that a widget is currently pressed.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Pressed;

/// Marker component indicating that a widget currently possesses keyboard or input focus.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Focused;

/// Marker component indicating that a widget is disabled and non-interactive.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Disabled;

/// Marker component indicating that an item is currently selected (e.g. Radio button).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Selected;

/// Tristate value representing check state for a checkbox.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CheckState {
    #[default]
    Unchecked,
    Checked,
    Indeterminate,
}
