//! Builder of the Material Design 3 checkbox: the setters and the state-derived roles.

use bevy::prelude::*;

use super::CheckboxBuilder;
use crate::interaction::state::CheckState;
use crate::theme::color::ColorRole;

impl CheckboxBuilder {
    /// Creates a new checkbox builder without a label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: Some(label.into()),
            state: CheckState::Unchecked,
            disabled: false,
        }
    }

    /// Creates an standalone checkbox builder without an attached label.
    pub fn standalone() -> Self {
        Self::default()
    }

    /// Sets the initial check state.
    pub fn with_state(mut self, state: CheckState) -> Self {
        self.state = state;
        self
    }

    /// Sets whether the checkbox is checked.
    pub fn checked(mut self, is_checked: bool) -> Self {
        self.state = if is_checked {
            CheckState::Checked
        } else {
            CheckState::Unchecked
        };
        self
    }

    /// Sets whether the checkbox is indeterminate.
    pub fn indeterminate(mut self, is_indeterminate: bool) -> Self {
        if is_indeterminate {
            self.state = CheckState::Indeterminate;
        }
        self
    }

    /// Sets whether the checkbox is disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub(super) fn glyph_char(state: CheckState) -> &'static str {
        match state {
            CheckState::Checked => "✓",
            CheckState::Indeterminate => "−",
            CheckState::Unchecked => "",
        }
    }

    pub(super) fn indicator_roles(state: CheckState) -> (ColorRole, ColorRole, ColorRole) {
        match state {
            CheckState::Checked | CheckState::Indeterminate => {
                (ColorRole::Primary, ColorRole::Primary, ColorRole::OnPrimary)
            }
            CheckState::Unchecked => (ColorRole::Surface, ColorRole::Outline, ColorRole::Primary),
        }
    }
}
