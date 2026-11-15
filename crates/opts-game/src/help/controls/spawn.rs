//! Spawns the controls table: the group headings, the notes and the binding rows.

use bevy::prelude::*;
use opts_gui::l10n::L10nKey;
use opts_gui::prelude::*;

use super::{ActionChip, Binding, GROUPS};
use crate::shell::keys;

/// A group heading (a message key).
fn heading(parent: &mut ChildSpawnerCommands, key: &'static str) {
    parent.spawn((
        Text::new(key),
        L10nKey(key),
        TextColor::default(),
        TypeStyleToken(TypeScale::TitleMedium),
        ForegroundRole(ColorRole::OnSurface),
    ));
}

/// A note below a heading (a message key).
fn note(parent: &mut ChildSpawnerCommands, key: &'static str) {
    parent.spawn((
        Text::new(key),
        L10nKey(key),
        TextColor::default(),
        TypeStyleToken(TypeScale::BodyMedium),
        ForegroundRole(ColorRole::OnSurfaceVariant),
    ));
}

/// One row of a group: the key chip and the action it does.
fn binding_row(parent: &mut ChildSpawnerCommands, binding: &Binding) {
    let Binding {
        shown: keys_text,
        action,
        ids,
        ..
    } = binding;
    parent
        .spawn(Node {
            column_gap: Val::Px(16.0),
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Node {
                    min_width: Val::Px(120.0),
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(2.0)),
                    ..default()
                },
                BackgroundRole(ColorRole::SurfaceContainerHighest),
                RadiusToken(RadiusLevel::Small),
            ))
            .with_children(|chip| {
                let mut text = chip.spawn((
                    Text::new(*keys_text),
                    TextColor::default(),
                    TypeStyleToken(TypeScale::Label),
                    ForegroundRole(ColorRole::OnSurface),
                ));
                if !ids.is_empty() {
                    text.insert(ActionChip(ids));
                }
            });
            row.spawn((
                Text::new(*action),
                L10nKey(action),
                TextColor::default(),
                TypeStyleToken(TypeScale::BodyMedium),
                ForegroundRole(ColorRole::OnSurfaceVariant),
            ));
        });
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
