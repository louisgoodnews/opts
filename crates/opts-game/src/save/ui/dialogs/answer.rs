//! The answers to the save overlay's dialogs: replace, delete or load, and closing the error dialog.

use super::*;

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

pub(super) fn answer(world: &mut World, choice: DialogChoice) {
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
        Some(Pending::Delete(id)) => match store(world).and_then(|s| s.delete(&id)) {
            Ok(()) => world.resource_mut::<SavesUi>().dirty = true,
            Err(e) => save_failed(world, &e),
        },
        Some(Pending::Load(id)) if session_running(world) => load(world, &id),
        _ => {}
    }
}
