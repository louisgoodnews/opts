//! The buttons of a save row and the row itself.

use super::details::row_details;
use super::*;

pub(super) fn row_button(
    parent: &mut ChildSpawnerCommands,
    key: &'static str,
    id: &str,
    action: RowAction,
    tag: &str,
    destructive: bool,
) -> Entity {
    let builder = if destructive {
        ButtonBuilder::text(key)
    } else {
        ButtonBuilder::tonal(key)
    };
    let button = builder.with_key(key).spawn_child(parent);
    parent.commands().entity(button).insert((
        SavesButton::Row(id.to_owned(), action),
        UiId::new(format!("saves.{tag}.{id}")),
    ));
    button
}

/// The buttons of a row: *Load*, *Overwrite* (when saving into a session), *Rename* and *Delete*.
pub(super) fn row_buttons(
    buttons: &mut ChildSpawnerCommands,
    entry: &SaveEntry,
    (mode, in_session): (SavesMode, bool),
) {
    let id = &entry.id;
    if entry.state.is_ok() {
        row_button(
            buttons,
            keys::SAVES_LOAD,
            id,
            RowAction::Load,
            "load",
            false,
        );
        if mode == SavesMode::Save && in_session {
            row_button(
                buttons,
                keys::SAVES_OVERWRITE,
                id,
                RowAction::Overwrite,
                "overwrite",
                false,
            );
        }
        row_button(
            buttons,
            keys::SAVES_RENAME,
            id,
            RowAction::Rename,
            "rename",
            true,
        );
    }
    row_button(
        buttons,
        keys::SAVES_DELETE,
        id,
        RowAction::Delete,
        "delete",
        true,
    );
}

pub(super) fn spawn_row(
    list: &mut ChildSpawnerCommands,
    entry: &SaveEntry,
    mode: SavesMode,
    issues: &[Issue],
    loc: &Localization,
    in_session: bool,
) {
    let id = &entry.id;
    list.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            padding: UiRect::all(Val::Px(12.0)),
            flex_shrink: 0.0,
            ..default()
        },
        BackgroundRole(ColorRole::SurfaceContainerHigh),
        RadiusToken(RadiusLevel::Medium),
        UiId::new(format!("saves.row.{id}")),
    ))
    .with_children(|row| {
        row_details(row, entry, issues, loc);
        row.spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(8.0),
            flex_wrap: FlexWrap::Wrap,
            ..default()
        })
        .with_children(|buttons| {
            row_buttons(buttons, entry, (mode, in_session));
        });
    });
}
