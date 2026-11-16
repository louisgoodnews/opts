//! Answering a dialog: a button or Esc sends the [`DialogChoice`] and closes the dialog.

use super::*;

/// Answers and closes a dialog when one of its buttons is activated.
pub fn dialog_button_system(
    mut commands: Commands,
    mut activated: MessageReader<Activated>,
    buttons: Query<&DialogButton>,
    dialogs: Query<&ConfirmDialog>,
    mut choices: MessageWriter<DialogChoice>,
) {
    for Activated(entity) in activated.read() {
        let Ok(button) = buttons.get(*entity) else {
            continue;
        };
        if let Ok(dialog) = dialogs.get(button.dialog) {
            choices.write(DialogChoice {
                tag: dialog.tag,
                choice: button.choice,
            });
            if !dialog.keep_open.contains(&button.choice) {
                commands.entity(button.dialog).try_despawn();
            }
        }
    }
}

/// Esc (the modal's cancel request) answers with [`CANCEL`] and closes the dialog.
pub fn dialog_cancel_system(
    mut commands: Commands,
    mut events: MessageReader<ModalEvent>,
    dialogs: Query<&ConfirmDialog, With<ModalRoot>>,
    mut choices: MessageWriter<DialogChoice>,
) {
    for ModalEvent::CancelRequested(modal) in events.read() {
        if let Ok(dialog) = dialogs.get(*modal) {
            choices.write(DialogChoice {
                tag: dialog.tag,
                choice: CANCEL,
            });
            commands.entity(*modal).try_despawn();
        }
    }
}
