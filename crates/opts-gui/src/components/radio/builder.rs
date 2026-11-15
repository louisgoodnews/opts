//! Builder of the Material Design 3 radio button: the setters and the state-derived roles.

use bevy::prelude::*;

use super::RadioBuilder;
use crate::theme::color::ColorRole;

impl RadioBuilder {
    /// Creates a new radio builder with an associated label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: Some(label.into()),
            is_selected: false,
            group_id: None,
            disabled: false,
        }
    }

    /// Creates a standalone radio builder without a label.
    pub fn standalone() -> Self {
        Self::default()
    }

    /// Sets whether the radio button is initially selected.
    pub fn selected(mut self, is_selected: bool) -> Self {
        self.is_selected = is_selected;
        self
    }

    /// Sets the group ID for mutual exclusion.
    pub fn with_group(mut self, group_id: u32) -> Self {
        self.group_id = Some(group_id);
        self
    }

    /// Sets whether the radio button is disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub(super) fn ring_border(is_selected: bool) -> ColorRole {
        if is_selected {
            ColorRole::Primary
        } else {
            ColorRole::Outline
        }
    }

    pub(super) fn dot_bg(is_selected: bool) -> ColorRole {
        if is_selected {
            ColorRole::Primary
        } else {
            ColorRole::Surface
        }
    }
}
