//! The name row of the saves overlay: the field for a new save's name and its error line.

use super::*;

pub(super) fn spawn_name_row(
    c: &mut ChildSpawnerCommands,
    mode: SavesMode,
    (label_text, placeholder): (String, String),
) {
    let name_row = c
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(12.0),
                flex_wrap: FlexWrap::Wrap,
                display: if mode == SavesMode::Save {
                    Display::Flex
                } else {
                    Display::None
                },
                ..default()
            },
            NameRow,
        ))
        .id();
    c.commands().entity(name_row).with_children(|r| {
        let field = TextFieldBuilder::outlined(label_text)
            .with_placeholder(placeholder)
            .with_max_characters(super::super::container::MAX_NAME_CHARS)
            .full_width()
            .spawn_child(r);
        r.commands()
            .entity(field)
            .insert((NameField, UiId::new("saves.name")));
        let save = ButtonBuilder::filled(keys::SAVES_SAVE)
            .with_key(keys::SAVES_SAVE)
            .spawn_child(r);
        r.commands()
            .entity(save)
            .insert((SavesButton::Save, UiId::new("saves.save")));
        let rename = ButtonBuilder::filled(keys::SAVES_RENAME_APPLY)
            .with_key(keys::SAVES_RENAME_APPLY)
            .spawn_child(r);
        r.commands().entity(rename).insert((
            SavesButton::ApplyRename,
            UiId::new("saves.rename-apply"),
            Node {
                display: Display::None,
                ..default()
            },
        ));
        r.spawn((
            Text::new(keys::SAVES_NAME_EMPTY),
            L10nKey(keys::SAVES_NAME_EMPTY),
            TextColor::default(),
            TypeStyleToken(TypeScale::BodyMedium),
            ForegroundRole(ColorRole::Error),
            Node {
                display: Display::None,
                ..default()
            },
            NameError,
            UiId::new("saves.name-error"),
        ));
    });
}
