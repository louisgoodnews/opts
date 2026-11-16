//! Confirmation dialogs: one small modal that names the consequence and offers the choices.
//!
//! Used by *Quit OPTS* and *Leave drive…*. The first button in tree order gets the focus when the
//! dialog opens, so the order is *safe choice first*. Esc and the backdrop's cancel request
//! answer with the choice [`CANCEL`]; the dialog closes itself in every case.

use bevy::prelude::*;
use opts_gui::components::modal::{ModalEvent, ModalRoot};
use opts_gui::interaction::activation::Activated;
use opts_gui::l10n::{L10nArgs, L10nKey};
use opts_gui::prelude::*;

/// The choice reported when the dialog is cancelled with Esc.
pub const CANCEL: &str = "cancel";

/// How a dialog button looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogButtonStyle {
    /// The safe, quiet choice (Cancel).
    Quiet,
    /// The expected choice.
    Primary,
    /// An irreversible choice (error colours).
    Destructive,
}

/// One button of a dialog.
#[derive(Debug, Clone, Copy)]
pub struct DialogButtonSpec {
    /// The choice id reported in [`DialogChoice`].
    pub choice: &'static str,
    /// Message key of the label.
    pub label_key: &'static str,
    /// Look.
    pub style: DialogButtonStyle,
}

/// What a dialog says and offers.
#[derive(Debug, Clone)]
pub struct DialogSpec {
    /// Identifies the dialog to the code that opened it (`shell.quit`).
    pub tag: &'static str,
    /// Message key of the title.
    pub title_key: &'static str,
    /// Message key of the body.
    pub body_key: &'static str,
    /// Variables of the body.
    pub body_args: L10nArgs,
    /// The buttons, safe choice first.
    pub buttons: Vec<DialogButtonSpec>,
    /// Choices that report [`DialogChoice`] but leave the dialog open (a folder chooser that
    /// has not answered yet).
    pub keep_open: Vec<&'static str>,
    /// An inline error under the body: message key and its variables.
    pub error: Option<(&'static str, L10nArgs)>,
}

/// Marker on the backdrop of a dialog.
#[derive(Component, Debug, Clone)]
pub struct ConfirmDialog {
    /// The dialog's tag.
    pub tag: &'static str,
    /// Choices that leave the dialog open.
    pub keep_open: Vec<&'static str>,
}

/// A button of a dialog.
#[derive(Component, Debug, Clone, Copy)]
pub struct DialogButton {
    /// The dialog backdrop.
    pub dialog: Entity,
    /// The choice id.
    pub choice: &'static str,
}

/// The user's answer; sent once, when the dialog closes.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DialogChoice {
    /// The dialog's tag.
    pub tag: &'static str,
    /// The chosen button's id, or [`CANCEL`].
    pub choice: &'static str,
}

mod answer;
mod spawn;
pub use answer::{dialog_button_system, dialog_cancel_system};
pub use spawn::spawn_confirm_dialog;

/// Registers the dialog message and systems.
pub fn build(app: &mut App) {
    app.add_message::<DialogChoice>()
        .add_systems(Update, (dialog_button_system, dialog_cancel_system));
}
