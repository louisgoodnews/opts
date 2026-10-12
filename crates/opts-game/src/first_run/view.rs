//! The first-run dialogs.

use bevy::prelude::*;
use opts_gui::l10n::L10nArgs;

use super::{FirstRun, Step, DIALOG_TAG};
use crate::first_run::decision::ChooseError;
use crate::shell::dialog::{
    spawn_confirm_dialog, ConfirmDialog, DialogButtonSpec, DialogButtonStyle, DialogSpec,
};
use crate::shell::keys;

/// Choice: use the folder that was found.
pub const USE: &str = "use";
/// Choice: choose another folder (from the *found* step).
pub const OTHER: &str = "other";
/// Choice: choose a folder.
pub const CHOOSE: &str = "choose";
/// Choice: quit.
pub const QUIT: &str = "quit";
/// Choice: welcome, pick a map and a vehicle.
pub const PICK: &str = "pick";
/// Choice: welcome, quick drive.
pub const QUICK: &str = "quick";

fn button(
    choice: &'static str,
    label_key: &'static str,
    style: DialogButtonStyle,
) -> DialogButtonSpec {
    DialogButtonSpec {
        choice,
        label_key,
        style,
    }
}

/// The dialog of a step.
pub fn spec_for(step: &Step, error: Option<&ChooseError>) -> DialogSpec {
    use DialogButtonStyle::{Primary, Quiet};
    let error = error.map(|e| match e {
        ChooseError::Cancelled => (keys::FIRSTRUN_CANCELLED, L10nArgs::default()),
        ChooseError::NotInstallation(path) => (
            keys::FIRSTRUN_WRONG_FOLDER,
            L10nArgs::one("path", path.display().to_string()),
        ),
    });
    match step {
        Step::Found(path) => DialogSpec {
            tag: DIALOG_TAG,
            title_key: keys::FIRSTRUN_FOUND_TITLE,
            body_key: keys::FIRSTRUN_FOUND_BODY,
            body_args: L10nArgs::one("path", path.display().to_string()),
            buttons: vec![
                button(USE, keys::FIRSTRUN_USE, Primary),
                button(OTHER, keys::FIRSTRUN_OTHER, Quiet),
                button(QUIT, keys::FIRSTRUN_QUIT, Quiet),
            ],
            keep_open: vec![OTHER],
            error,
        },
        Step::Choose => DialogSpec {
            tag: DIALOG_TAG,
            title_key: keys::FIRSTRUN_CHOOSE_TITLE,
            body_key: keys::FIRSTRUN_CHOOSE_BODY,
            body_args: L10nArgs::default(),
            buttons: vec![
                button(CHOOSE, keys::FIRSTRUN_CHOOSE_ACTION, Primary),
                button(QUIT, keys::FIRSTRUN_QUIT, Quiet),
            ],
            keep_open: vec![CHOOSE],
            error,
        },
        Step::Welcome => DialogSpec {
            tag: DIALOG_TAG,
            title_key: keys::FIRSTRUN_WELCOME_TITLE,
            body_key: keys::FIRSTRUN_WELCOME_BODY,
            body_args: L10nArgs::default(),
            buttons: vec![
                button(PICK, keys::FIRSTRUN_PICK, Primary),
                button(QUICK, keys::FIRSTRUN_QUICK, Quiet),
            ],
            keep_open: Vec::new(),
            error: None,
        },
    }
}

/// Keeps the open dialog in step with [`FirstRun`]: one dialog per step and error.
pub fn sync_first_run_dialog(
    mut commands: Commands,
    first_run: Res<FirstRun>,
    open: Query<Entity, With<ConfirmDialog>>,
    dialogs: Query<&ConfirmDialog>,
    mut shown: Local<Option<(Option<Step>, Option<ChooseError>)>>,
) {
    let want = (first_run.step.clone(), first_run.error.clone());
    let is_open = dialogs.iter().any(|d| d.tag == DIALOG_TAG);
    if shown.as_ref() == Some(&want) && (is_open || want.0.is_none()) {
        return;
    }
    for entity in &open {
        if dialogs.get(entity).is_ok_and(|d| d.tag == DIALOG_TAG) {
            commands.entity(entity).despawn();
        }
    }
    if let Some(step) = &want.0 {
        spawn_confirm_dialog(&mut commands, spec_for(step, want.1.as_ref()));
    }
    *shown = Some(want);
}
