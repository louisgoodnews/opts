//! Input of the settings page: what the controls do, and the revert prompt of risky settings.

use bevy::prelude::*;
use opts_gui::components::modal::ModalEvent;
use opts_gui::interaction::events::{ButtonClicked, ComboboxSelected, SwitchToggled, TextChanged};

use crate::settings_registry::{Category, SettingKind, SettingsApi};

use super::page::{SettingsPageState, SettingsSearch};

mod change;
mod handlers;
mod ops;
mod params;
mod rebind;
mod revert;

pub use handlers::handle_page_input;
pub use rebind::capture_rebind;
pub use revert::revert_prompt_system;

/// A combo box that sets a text setting: the stored text of each option, in order.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct SettingCombo {
    /// Setting key.
    pub key: &'static str,
    /// The stored text of every option (the first, empty, is the default).
    pub values: Vec<String>,
}

/// What a control of the page does when activated.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub enum SettingOp {
    /// Sets a value.
    Set {
        /// Setting key.
        key: &'static str,
        /// Stored text.
        value: String,
    },
    /// Steps a number or time up or down.
    Step {
        /// Setting key.
        key: &'static str,
        /// -1 or 1.
        dir: i32,
    },
    /// Restores the default.
    Reset {
        /// Setting key.
        key: &'static str,
    },
    /// Starts an action or a chooser: announced as [`SettingActionRequested`].
    Run {
        /// Setting key.
        key: &'static str,
    },
    /// Shows a category.
    Category(Category),
}

/// A switch bound to a boolean setting.
#[derive(Component, Debug, Clone, Copy)]
pub struct SettingSwitch(pub &'static str);

/// An action button or path chooser of the page was used; the owner of the feature handles it.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettingActionRequested {
    /// The key of the action or path setting.
    pub key: &'static str,
}

/// The risky change waiting for the user's confirmation.
#[derive(Resource, Debug)]
pub struct PendingRevert {
    /// Setting key.
    pub key: String,
    /// Value to return to.
    pub old: String,
    /// The value being tried.
    pub new: String,
    /// Seconds left.
    pub remaining: f32,
    /// The modal.
    pub modal: Entity,
    /// The countdown text.
    pub countdown: Entity,
}

/// Buttons of the revert prompt.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevertButton {
    /// Keep the new value.
    Keep,
    /// Go back.
    Revert,
}
