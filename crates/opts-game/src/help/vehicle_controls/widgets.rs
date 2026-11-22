//! The small widgets of the vehicle controls list: a heading, a note and a key chip, built from the
//! `opts_gui` tokens.

use bevy::prelude::*;
use opts_gui::l10n::L10nKey;
use opts_gui::prelude::*;

use crate::help::controls::{ActionChip, KeyChip, KEY_CHIP_WIDTH};

pub fn heading(parent: &mut ChildSpawnerCommands, key: &'static str) {
    parent.spawn((
        Text::new(key),
        L10nKey(key),
        TextColor::default(),
        TypeStyleToken(TypeScale::TitleMedium),
        ForegroundRole(ColorRole::OnSurface),
    ));
}

pub fn note(parent: &mut ChildSpawnerCommands, key: &'static str) {
    parent.spawn((
        Text::new(key),
        L10nKey(key),
        TextColor::default(),
        TypeStyleToken(TypeScale::BodyMedium),
        ForegroundRole(ColorRole::OnSurfaceVariant),
    ));
}

/// A key chip (`width` is [`KEY_CHIP_WIDTH`] in a key row, `Val::Auto` for a free-standing name).
/// A key chip: `width` is [`super::super::controls::KEY_CHIP_WIDTH`] in a key row (one width for
/// every row, ISS-055) and `Val::Auto` for a free-standing name. `ids` are the rebindable actions
/// behind it (empty for keys that cannot move).
pub fn chip(
    parent: &mut ChildSpawnerCommands,
    text: &str,
    width: Val,
    ids: &'static [&'static str],
) {
    parent
        .spawn((
            Node {
                width,
                padding: UiRect::axes(Val::Px(8.0), Val::Px(2.0)),
                ..default()
            },
            KeyChip,
            BackgroundRole(ColorRole::SurfaceContainerHighest),
            RadiusToken(RadiusLevel::Small),
        ))
        .with_children(|c| {
            let mut t = c.spawn((
                Text::new(text),
                TextColor::default(),
                TypeStyleToken(TypeScale::Label),
                ForegroundRole(ColorRole::OnSurface),
            ));
            if !ids.is_empty() {
                t.insert(ActionChip(ids));
            }
        });
}

/// One row of a key table: the key chip, then the action it does (`action` is a message key).
pub fn key_row(
    parent: &mut ChildSpawnerCommands,
    shown: &str,
    ids: &'static [&'static str],
    action: &'static str,
) {
    parent
        .spawn(Node {
            column_gap: Val::Px(16.0),
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|row| {
            chip(row, shown, Val::Px(KEY_CHIP_WIDTH), ids);
            row.spawn((
                Text::new(action),
                L10nKey(action),
                TextColor::default(),
                TypeStyleToken(TypeScale::BodyMedium),
                ForegroundRole(ColorRole::OnSurfaceVariant),
            ));
        });
}
