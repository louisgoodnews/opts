//! Interaction state components shared by every widget.
//!
//! The pointer states are Bevy's own: [`Hovered`] (`bevy::picking::hover`, true while the pointer
//! is over the control or one of its descendants) and [`Pressed`] (`bevy::ui`, "held down").
//! The remaining states are defined here once; the visual treatment of all of them lives in
//! [`crate::interaction::layer`].

use bevy::prelude::*;
use std::borrow::Cow;

pub use bevy::picking::hover::Hovered;
pub use bevy::ui::Pressed;

/// Marker component indicating that a widget currently possesses keyboard or input focus.
///
/// Mirrors the `InputFocus` resource (see [`crate::interaction::focus`]); do not rely on it being
/// the only writer, use [`crate::interaction::focus::FocusEvent`] to move focus.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Focused;

/// Marker component indicating that a widget is disabled and non-interactive.
///
/// A disabled control must say why: add a [`DisabledReason`] (shown as tooltip) or mark the place
/// with the `NotAvailableYet` marker (batch B5). The CI rule tests fail on a disabled control with
/// neither.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Disabled;

/// The reason a control is disabled, shown as tooltip on hover and keyboard focus.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct DisabledReason(pub Cow<'static, str>);

impl DisabledReason {
    /// Creates a reason from any string.
    pub fn new(reason: impl Into<Cow<'static, str>>) -> Self {
        Self(reason.into())
    }
}

/// Marker component indicating that an item is currently selected (e.g. Radio button).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Selected;

/// Validation error of a control: drawn as error border and (where the widget has room) message.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct ErrorState(pub Cow<'static, str>);

impl ErrorState {
    /// Creates an error state with a message.
    pub fn new(message: impl Into<Cow<'static, str>>) -> Self {
        Self(message.into())
    }
}

/// Tristate value representing check state for a checkbox.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CheckState {
    #[default]
    Unchecked,
    Checked,
    Indeterminate,
}

/// The seven visual states of a control, as the state layer system sees them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UiState {
    /// Pointer is over the control.
    pub hovered: bool,
    /// Control holds focus.
    pub focused: bool,
    /// Pointer button held down on the control.
    pub pressed: bool,
    /// Control is selected.
    pub selected: bool,
    /// Control is disabled.
    pub disabled: bool,
    /// Control shows a validation error.
    pub error: bool,
}

impl UiState {
    /// Opacity of the state layer (M3: hover 8 %, focus 10 %, pressed 10 %); disabled controls
    /// show no layer.
    pub fn layer_alpha(&self) -> f32 {
        if self.disabled {
            0.0
        } else if self.pressed || self.focused {
            LAYER_PRESSED
        } else if self.hovered {
            LAYER_HOVER
        } else {
            0.0
        }
    }
}

/// State layer opacity of a hovered control.
pub const LAYER_HOVER: f32 = 0.08;
/// State layer opacity of a focused control.
pub const LAYER_FOCUS: f32 = 0.10;
/// State layer opacity of a pressed control.
pub const LAYER_PRESSED: f32 = 0.10;
/// Share of content that stays visible on a disabled control (M3: 38 %).
pub const DISABLED_CONTENT: f32 = 0.38;
/// Share of the container that stays visible on a disabled control (M3: 12 %).
pub const DISABLED_CONTAINER: f32 = 0.12;
