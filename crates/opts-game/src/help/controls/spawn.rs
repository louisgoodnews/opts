//! Spawns the controls table: the group headings, the notes and the binding rows.

use bevy::prelude::*;

use super::{Binding, GROUPS};
use crate::help::vehicle_controls::widgets::{heading, key_row, note};
use crate::shell::keys;

/// One row of a group: the key chip and the action it does.
fn binding_row(parent: &mut ChildSpawnerCommands, binding: &Binding) {
    key_row(parent, binding.shown, binding.ids, binding.action);
}

/// Spawns the table into `parent`.
pub fn spawn_controls(parent: &mut ChildSpawnerCommands) {
    // The vehicle system keys are rebindable (Settings, Controls), so the table does not list a
    // fixed key for them; this note points to the defaults and the page.
    heading(parent, keys::HELP_GROUP_VEHICLE_KEYS);
    note(parent, keys::HELP_VEHICLE_KEYS_NOTE);
    crate::help::vehicle_controls::spawn_list(parent);
    heading(parent, keys::HELP_GROUP_MOUSE_DRIVING);
    note(parent, keys::HELP_MOUSE_DRIVING_NOTE);
    for (group, rows) in GROUPS {
        heading(parent, group);
        for binding in *rows {
            binding_row(parent, binding);
        }
    }
}
