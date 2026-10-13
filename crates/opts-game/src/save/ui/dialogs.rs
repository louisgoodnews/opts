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

fn args(name: &str) -> L10nArgs {
    L10nArgs(vec![("name".to_owned(), Arg::from(name.to_owned()))])
}

fn ask(
    world: &mut World,
    tag: &'static str,
    title: &'static str,
    body: &'static str,
    confirm: &'static str,
    name: &str,
    destructive: bool,
) {
    let spec = DialogSpec {
        tag,
        title_key: title,
        body_key: body,
        body_args: args(name),
        buttons: vec![
            DialogButtonSpec {
                choice: CANCEL,
                label_key: keys::CANCEL,
                style: DialogButtonStyle::Quiet,
            },
            DialogButtonSpec {
                choice: YES,
                label_key: confirm,
                style: if destructive {
                    DialogButtonStyle::Destructive
                } else {
                    DialogButtonStyle::Primary
                },
            },
        ],
        keep_open: Vec::new(),
        error: None,
    };
    let mut commands = world.commands();
    spawn_confirm_dialog(&mut commands, spec);
    world.flush();
}

/// Asks before a save is replaced.
pub fn ask_overwrite(world: &mut World, id: Option<String>, name: String) {
    world.resource_mut::<SavesUi>().pending = Some(Pending::Overwrite(id, name.clone()));
    ask(
        world,
        OVERWRITE_DIALOG,
        keys::SAVES_OVERWRITE_TITLE,
        keys::SAVES_OVERWRITE_BODY,
        keys::SAVES_OVERWRITE,
        &name,
        true,
    );
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

fn name_of(world: &World, id: &str) -> String {
    store(world)
        .ok()
        .and_then(|s| s.read(id).ok())
        .map_or_else(|| id.to_owned(), |l| l.file.meta.name)
}

/// A row's button.
pub fn row_action(world: &mut World, id: &str, action: RowAction) {
    match action {
        RowAction::Load => {
            if *world.resource::<State<AppState>>().get() == AppState::InGame {
                let name = name_of(world, id);
                world.resource_mut::<SavesUi>().pending = Some(Pending::Load(id.to_owned()));
                ask(
                    world,
                    LOAD_DIALOG,
                    keys::SAVES_LOAD_TITLE,
                    keys::SAVES_LOAD_BODY,
                    keys::SAVES_LOAD,
                    &name,
                    true,
                );
            } else {
                load(world, id);
            }
        }
        RowAction::Overwrite => {
            let name = name_of(world, id);
            ask_overwrite(world, Some(id.to_owned()), name);
        }
        RowAction::Rename => {
            let name = name_of(world, id);
            set_field_text(world, &name);
            let mut ui = world.resource_mut::<SavesUi>();
            ui.renaming = Some(id.to_owned());
            ui.name_error = None;
        }
        RowAction::Delete => {
            let name = name_of(world, id);
            world.resource_mut::<SavesUi>().pending = Some(Pending::Delete(id.to_owned()));
            ask(
                world,
                DELETE_DIALOG,
                keys::SAVES_DELETE_TITLE,
                keys::SAVES_DELETE_BODY,
                keys::SAVES_DELETE,
                &name,
                true,
            );
        }
    }
}

/// Starts restoring; a refusal opens the error dialog.
pub fn load(world: &mut World, id: &str) {
    if let Err(e) = begin_restore(world, id) {
        tracing::warn!(%id, "load refused: {e}");
        let name = name_of(world, id);
        open_error(world, &name, &e);
    }
}

/// The error template for a load that cannot start: what happened (title), why (body), what you
/// can do (*Choose another*, *Back*); the technical text is in the body's last sentence.
fn open_error(world: &mut World, name: &str, error: &RestoreError) {
    let mut body_args = vec![("name".to_owned(), Arg::from(name.to_owned()))];
    let body = match error {
        RestoreError::Save(SaveError::NewerFormat { found, .. }) => {
            body_args.push(("found".to_owned(), Arg::from(found.to_string())));
            keys::LOAD_ERR_NEWER
        }
        RestoreError::Save(SaveError::Corrupt(_) | SaveError::NotASave) => {
            body_args.push(("error".to_owned(), Arg::from(error.to_string())));
            keys::LOAD_ERR_CORRUPT
        }
        RestoreError::Blocked(issues) => {
            let blocking = |kind| {
                issues
                    .iter()
                    .filter(|i| i.blocking && i.kind == kind)
                    .map(|i| i.subject.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let map = blocking(IssueKind::MissingMap);
            if map.is_empty() {
                body_args.push((
                    "vehicles".to_owned(),
                    Arg::from(blocking(IssueKind::MissingVehicle)),
                ));
                keys::LOAD_ERR_MISSING_VEHICLES
            } else {
                body_args.push(("map".to_owned(), Arg::from(map)));
                keys::LOAD_ERR_MISSING_MAP
            }
        }
        RestoreError::Save(_) => {
            body_args.push(("error".to_owned(), Arg::from(error.to_string())));
            keys::LOAD_ERR_OTHER
        }
    };
    let spec = DialogSpec {
        tag: ERROR_DIALOG,
        title_key: keys::LOAD_ERROR_TITLE,
        body_key: body,
        body_args: L10nArgs(body_args),
        buttons: vec![
            DialogButtonSpec {
                choice: ANOTHER,
                label_key: keys::LOAD_CHOOSE_ANOTHER,
                style: DialogButtonStyle::Primary,
            },
            DialogButtonSpec {
                choice: BACK,
                label_key: keys::LOAD_BACK,
                style: DialogButtonStyle::Quiet,
            },
        ],
        keep_open: Vec::new(),
        error: None,
    };
    let mut commands = world.commands();
    spawn_confirm_dialog(&mut commands, spec);
    world.flush();
}

/// The answers of the overlay's dialogs.
pub fn dialog_choices(mut choices: MessageReader<DialogChoice>, mut commands: Commands) {
    for choice in choices.read().copied() {
        if !matches!(
            choice.tag,
            OVERWRITE_DIALOG | DELETE_DIALOG | LOAD_DIALOG | ERROR_DIALOG
        ) {
            continue;
        }
        commands.queue(move |world: &mut World| answer(world, choice));
    }
}

fn answer(world: &mut World, choice: DialogChoice) {
    let pending = world.resource_mut::<SavesUi>().pending.take();
    if choice.tag == ERROR_DIALOG {
        // *Choose another* leaves the list as it is; *Back* closes the overlay.
        if choice.choice != ANOTHER {
            close_overlay(world);
        }
        return;
    }
    if choice.choice != YES {
        return;
    }
    match pending {
        Some(Pending::Overwrite(id, name)) => {
            let request = SaveRequest {
                name,
                id,
                overwrite: true,
            };
            match save_session(world, &request) {
                Ok(saved) => {
                    saved_toast(world, &saved.name);
                    world.resource_mut::<SavesUi>().dirty = true;
                }
                Err(e) => save_failed(world, &e),
            }
        }
        Some(Pending::Delete(id)) => {
            match store(world).and_then(|s| s.delete(&id)) {
                Ok(()) => world.resource_mut::<SavesUi>().dirty = true,
                Err(e) => save_failed(world, &e),
            }
        }
        Some(Pending::Load(id)) if session_running(world) => load(world, &id),
        _ => {}
    }
}
