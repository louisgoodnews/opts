//! Builder of the Material Design 3 button: the variant constructors and the fluent setters.

use std::sync::Arc;

use bevy::prelude::*;

use super::{ButtonClickHandler, ButtonVariant};

/// Builder for constructing Material Design 3 buttons with fluent configuration.
///
/// # Example
/// ```rust
/// use opts_gui::prelude::*;
/// use bevy::prelude::*;
///
/// fn spawn_my_button(mut commands: Commands) {
///     ButtonBuilder::filled("Click Me")
///         .on_click(|_commands, _entity| {
///             // Action
///         })
///         .spawn(&mut commands);
/// }
/// ```
#[derive(Clone)]
pub struct ButtonBuilder {
    pub label: String,
    pub variant: ButtonVariant,
    pub icon: Option<Handle<Image>>,
    pub is_disabled: bool,
    pub on_click: Option<ButtonClickHandler>,
    /// Message key of the label: the text follows the active locale.
    pub label_key: Option<&'static str>,
}


impl ButtonBuilder {
    /// Creates a new filled button builder.
    pub fn filled(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            variant: ButtonVariant::Filled,
            icon: None,
            is_disabled: false,
            on_click: None,
            label_key: None,
        }
    }

    /// Creates a new elevated button builder.
    pub fn elevated(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            variant: ButtonVariant::Elevated,
            icon: None,
            is_disabled: false,
            on_click: None,
            label_key: None,
        }
    }

    /// Creates a new filled tonal button builder.
    pub fn tonal(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            variant: ButtonVariant::FilledTonal,
            icon: None,
            is_disabled: false,
            on_click: None,
            label_key: None,
        }
    }

    /// Creates a new outlined button builder.
    pub fn outlined(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            variant: ButtonVariant::Outlined,
            icon: None,
            is_disabled: false,
            on_click: None,
            label_key: None,
        }
    }

    /// Creates a new text button builder.
    pub fn text(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            variant: ButtonVariant::Text,
            icon: None,
            is_disabled: false,
            on_click: None,
            label_key: None,
        }
    }

    /// Creates a circular icon button builder.
    pub fn icon(icon: Handle<Image>) -> Self {
        Self {
            label: String::new(),
            variant: ButtonVariant::Icon,
            icon: Some(icon),
            is_disabled: false,
            on_click: None,
            label_key: None,
        }
    }

    /// Makes the label a message key (`label` stays the fallback text until it is resolved).
    pub fn with_key(mut self, key: &'static str) -> Self {
        self.label_key = Some(key);
        self
    }

    /// Attaches an optional leading icon asset.
    pub fn with_icon(mut self, icon: Handle<Image>) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Marks the button as disabled (non-interactive).
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.is_disabled = disabled;
        self
    }

    /// Attaches an `on_click` action callback that runs whenever the button is activated.
    pub fn on_click<F>(mut self, callback: F) -> Self
    where
        F: Fn(&mut Commands, Entity) + Send + Sync + 'static,
    {
        self.on_click = Some(Arc::new(callback));
        self
    }
}
