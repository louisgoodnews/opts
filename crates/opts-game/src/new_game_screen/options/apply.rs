//! Writing the equipment dialog's edits into the choices: switches, choice boxes and numbers, and closing on Escape.

use super::*;

/// Writes a toggled switch into the choices (a value equal to the default is no override).
pub fn apply_switches(
    mut toggled: MessageReader<SwitchToggled>,
    switches: Query<&OptionSwitch>,
    mut setup: ResMut<NewGameSetup>,
) {
    for t in toggled.read() {
        let Ok(s) = switches.get(t.entity) else {
            continue;
        };
        set_override(&mut setup, &s.name, s.default, i32::from(t.is_on));
    }
}

/// Writes a chosen value of a choice box into the choices.
pub fn apply_choices(
    mut chosen: MessageReader<ComboboxSelected>,
    boxes: Query<&OptionChoice>,
    mut setup: ResMut<NewGameSetup>,
) {
    for c in chosen.read() {
        let Ok(b) = boxes.get(c.entity) else {
            continue;
        };
        let Some(value) = b.values.get(c.selected_index).copied() else {
            continue;
        };
        set_override(&mut setup, &b.name, b.default, value);
    }
}

/// Writes an edited number into the choices (text that is not a number in range is ignored).
pub fn apply_numbers(
    mut edited: MessageReader<TextChanged>,
    fields: Query<&OptionNumber>,
    mut setup: ResMut<NewGameSetup>,
) {
    for t in edited.read() {
        let Ok(f) = fields.get(t.entity) else {
            continue;
        };
        let Ok(value) = t.text.trim().parse::<i32>() else {
            continue;
        };
        if (f.min..=f.max).contains(&value) {
            set_override(&mut setup, &f.name, f.default, value);
        }
    }
}

/// A value equal to the livery's own is no override.
pub(super) fn set_override(setup: &mut NewGameSetup, name: &str, default: i32, value: i32) {
    if value == default {
        setup.custom_setvars.remove(name);
    } else {
        setup.custom_setvars.insert(name.to_string(), value);
    }
}

/// Closes the dialog on Escape.
pub fn close_on_cancel(
    mut events: MessageReader<ModalEvent>,
    modals: Query<Entity, With<OptionsModal>>,
    mut commands: Commands,
) {
    for ModalEvent::CancelRequested(e) in events.read() {
        if modals.contains(*e) {
            commands.entity(*e).despawn();
        }
    }
}
