//! The questions and errors of the save overlay: overwrite, delete, load over a running drive, and
//! the error template of a load that cannot start.

use bevy::prelude::*;
use opts_gui::components::toast::ToastSeverity;
use opts_gui::l10n::fluent::Arg;
use opts_gui::l10n::L10nArgs;

use super::{close_overlay, set_field_text, RowAction, SavesUi};
use crate::app_state::AppState;
use crate::save::container::SaveError;
use crate::save::ops::{save_session, session_running, store, SaveRequest};
use crate::save::registry::IssueKind;
use crate::save::restore::{begin_restore, RestoreError};
use crate::shell::dialog::{
    spawn_confirm_dialog, DialogButtonSpec, DialogButtonStyle, DialogChoice, DialogSpec, CANCEL,
};
use crate::shell::keys;
use crate::shell::notify::NotifyExt;

mod answer;
mod error;
mod questions;

pub use answer::dialog_choices;
use error::open_error;
pub use questions::{ask_overwrite, load, row_action};

/// Tag of the overwrite question.
pub const OVERWRITE_DIALOG: &str = "saves.overwrite";
/// Tag of the delete question.
pub const DELETE_DIALOG: &str = "saves.delete";
/// Tag of the load question (a drive is running).
pub const LOAD_DIALOG: &str = "saves.load";
/// Tag of the error dialog of a load.
pub const ERROR_DIALOG: &str = "saves.error";
const YES: &str = "yes";
const ANOTHER: &str = "another";
const BACK: &str = "back";

/// What a question is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pending {
    /// Replace a save (its id, or `None` for a name the store will resolve) with this name.
    Overwrite(Option<String>, String),
    /// Delete this save.
    Delete(String),
    /// Load this save over the running drive.
    Load(String),
}

/// A toast that the drive was saved.
pub fn saved_toast(world: &mut World, name: &str) {
    world.commands().notify(
        ToastSeverity::Info,
        keys::SAVE_SAVED_TOAST,
        vec![("name", name.to_owned())],
    );
    world.flush();
}

/// A save failed: an error toast and the reason in the log.
pub fn save_failed(world: &mut World, error: &SaveError) {
    tracing::warn!("save failed: {error}");
    world.commands().notify(
        ToastSeverity::Error,
        keys::SAVE_FAILED_TOAST,
        vec![("error", error.to_string())],
    );
    world.flush();
}
