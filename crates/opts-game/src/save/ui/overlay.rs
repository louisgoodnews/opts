//! The saves overlay: its frame, title, list and buttons, and the label helper of its rows.

use super::name_row::spawn_name_row;
use super::*;

pub(super) fn label(
    card: &mut ChildSpawnerCommands,
    key: &'static str,
    scale: TypeScale,
    role: ColorRole,
) -> Entity {
    card.spawn((
        Text::new(key),
        L10nKey(key),
        TextColor::default(),
        TypeStyleToken(scale),
        ForegroundRole(role),
    ))
    .id()
}

pub(super) fn spawn_overlay(
    commands: &mut Commands,
    mode: SavesMode,
    (label_text, placeholder): (String, String),
) {
    let (root, card) = ModalBuilder::new()
        .with_size_percent(80.0, 85.0)
        .with_width_range(560.0, 900.0)
        .spawn(commands);
    commands
        .entity(root)
        .insert((SavesOverlay, Name::new("SavesOverlay")));
    commands.entity(card).with_children(|c| {
        let title = match mode {
            SavesMode::Save => keys::SAVES_TITLE_SAVE,
            SavesMode::Load => keys::SAVES_TITLE_LOAD,
        };
        label(c, title, TypeScale::TitleLarge, ColorRole::OnSurface);
        label(
            c,
            keys::SAVES_NOTE,
            TypeScale::BodyMedium,
            ColorRole::OnSurfaceVariant,
        );
        spawn_name_row(c, mode, (label_text, placeholder));
        c.spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            SavesList,
            UiId::new("saves.list"),
        ));
        let close = ButtonBuilder::tonal(keys::HELP_CLOSE)
            .with_key(keys::HELP_CLOSE)
            .spawn_child(c);
        c.commands().entity(close).insert((
            SavesButton::Close,
            UiId::new("saves.close"),
            Node {
                align_self: AlignSelf::FlexEnd,
                ..default()
            },
        ));
    });
}
