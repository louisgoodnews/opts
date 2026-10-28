//! Button, name-field, and cancel handlers for the *Save game* / *Load game* overlay.

use super::*;

/// Esc closes the overlay (the topmost modal answers first, so a dialog above it closes alone).
pub fn close_on_cancel(
    mut commands: Commands,
    mut cancels: MessageReader<ModalEvent>,
    overlays: Query<Entity, With<SavesOverlay>>,
) {
    for ModalEvent::CancelRequested(modal) in cancels.read() {
        if overlays.contains(*modal) {
            commands.entity(*modal).despawn();
        }
    }
}

pub(crate) fn close_overlay(world: &mut World) {
    let roots: Vec<Entity> = world
        .query_filtered::<Entity, With<SavesOverlay>>()
        .iter(world)
        .collect();
    for e in roots {
        world.entity_mut(e).despawn();
    }
}

/// Clicks on the overlay's own buttons.
pub fn handle_buttons(
    mut activated: MessageReader<Activated>,
    buttons: Query<&SavesButton>,
    mut commands: Commands,
) {
    for Activated(entity) in activated.read() {
        let Ok(button) = buttons.get(*entity).cloned() else {
            continue;
        };
        commands.queue(move |world: &mut World| match button {
            SavesButton::Close => close_overlay(world),
            SavesButton::Save => submit_name(world),
            SavesButton::ApplyRename => apply_rename(world),
            SavesButton::Row(id, action) => dialogs::row_action(world, &id, action),
        });
    }
}

fn field_text(world: &mut World) -> String {
    world
        .query_filtered::<&M3TextField, With<NameField>>()
        .iter(world)
        .next()
        .map(|f| f.text.clone())
        .unwrap_or_default()
}

pub(crate) fn set_field_text(world: &mut World, text: &str) {
    for mut f in world
        .query_filtered::<&mut M3TextField, With<NameField>>()
        .iter_mut(world)
    {
        f.text = text.to_owned();
    }
}

/// *Save* with the name in the field: asks before replacing a save of the same name.
fn submit_name(world: &mut World) {
    let name = field_text(world);
    let request = SaveRequest {
        name: name.clone(),
        id: None,
        overwrite: false,
    };
    match save_session(world, &request) {
        Ok(saved) => {
            set_field_text(world, "");
            dialogs::saved_toast(world, &saved.name);
            let mut ui = world.resource_mut::<SavesUi>();
            ui.dirty = true;
            ui.name_error = None;
        }
        Err(SaveError::Exists(id)) => dialogs::ask_overwrite(world, Some(id), name),
        Err(SaveError::InvalidName(_)) => {
            world.resource_mut::<SavesUi>().name_error = Some(keys::SAVES_NAME_EMPTY);
        }
        Err(e) => dialogs::save_failed(world, &e),
    }
}

fn apply_rename(world: &mut World) {
    let Some(id) = world.resource_mut::<SavesUi>().renaming.take() else {
        return;
    };
    let name = field_text(world);
    match super::super::ops::store(world).and_then(|s| s.rename(&id, &name)) {
        Ok(()) => {
            set_field_text(world, "");
            let mut ui = world.resource_mut::<SavesUi>();
            ui.dirty = true;
            ui.name_error = None;
        }
        Err(SaveError::InvalidName(_)) => {
            let mut ui = world.resource_mut::<SavesUi>();
            ui.renaming = Some(id);
            ui.name_error = Some(keys::SAVES_NAME_EMPTY);
        }
        Err(e) => dialogs::save_failed(world, &e),
    }
}

/// Keeps the name row, its buttons and its error in step with the overlay's state.
pub fn sync_name_row(
    ui: Res<SavesUi>,
    mut rows: Query<&mut Node, (With<NameRow>, Without<NameError>)>,
    mut errors: Query<&mut Node, (With<NameError>, Without<NameRow>)>,
    buttons: Query<(&SavesButton, Entity)>,
    mut nodes: Query<&mut Node, (Without<NameRow>, Without<NameError>)>,
) {
    if !ui.is_changed() {
        return;
    }
    let visible = ui.mode == SavesMode::Save || ui.renaming.is_some();
    for mut node in &mut rows {
        node.display = if visible {
            Display::Flex
        } else {
            Display::None
        };
    }
    for mut node in &mut errors {
        node.display = if ui.name_error.is_some() {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (button, entity) in &buttons {
        let show = match button {
            SavesButton::Save => ui.renaming.is_none(),
            SavesButton::ApplyRename => ui.renaming.is_some(),
            _ => continue,
        };
        if let Ok(mut node) = nodes.get_mut(entity) {
            node.display = if show { Display::Flex } else { Display::None };
        }
    }
}
