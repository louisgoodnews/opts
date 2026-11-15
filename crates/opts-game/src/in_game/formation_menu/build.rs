//! The widgets of the formation editor page.

use bevy::prelude::*;
use opts_gui::prelude::*;

use super::state::{Act, FormationEditor, FormationList, FormationRoot, ListKind};

mod columns;
use columns::{summary_column, train_column};
mod tabs;
use tabs::{list_tabs, search_field};
mod catalogue;
use catalogue::catalogue_column;

pub(super) fn label(
    p: &mut ChildSpawnerCommands,
    text: impl Into<String>,
    size: f32,
    role: ColorRole,
) {
    p.spawn((
        Text::new(text.into()),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        ForegroundRole(role),
    ));
}

pub(super) fn button(p: &mut ChildSpawnerCommands, text: &str, selected: bool, act: Act) -> Entity {
    let builder = if selected {
        ButtonBuilder::filled(text)
    } else {
        ButtonBuilder::tonal(text)
    };
    let e = builder.spawn_child(p);
    p.commands().entity(e).insert(act);
    e
}

pub(super) fn scroll_list(
    p: &mut ChildSpawnerCommands,
    which: usize,
    offset: f32,
    fill: impl FnOnce(&mut ChildSpawnerCommands),
) {
    p.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            width: Val::Percent(100.0),
            flex_grow: 1.0,
            min_height: Val::Px(0.0),
            overflow: Overflow::scroll_y(),
            padding: UiRect::right(Val::Px(6.0)),
            ..default()
        },
        ScrollPosition(Vec2::new(0.0, offset)),
        FormationList(which),
    ))
    .with_children(fill);
}

fn column(p: &mut ChildSpawnerCommands, width_percent: f32) -> Entity {
    p.spawn(Node {
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(8.0),
        width: Val::Percent(width_percent),
        height: Val::Percent(100.0),
        min_height: Val::Px(0.0),
        ..default()
    })
    .id()
}

pub(super) fn id(commands: &mut Commands, e: Entity, name: &str) {
    commands.entity(e).insert(UiId::new(name.to_owned()));
}

/// What the entry-point column shows.
pub(super) struct Entries<'a> {
    /// Free entry points: record number and label.
    pub free: &'a [(u32, String)],
    /// The chosen one.
    pub chosen: Option<u32>,
    /// Entries resolved so far and whether the index has finished.
    pub progress: (usize, bool),
}

/// Builds the editor modal.
pub(super) fn build_formation(commands: &mut Commands, ed: &FormationEditor, entries: &Entries) {
    let (root, content) = ModalBuilder::new()
        .with_size_percent(94.0, 92.0)
        .with_width_range(900.0, 1700.0)
        .spawn(commands);
    commands.entity(root).insert(FormationRoot);
    commands.entity(content).with_children(|p| {
        p.spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(16.0),
            ..default()
        })
        .with_children(|tabs| {
            label(tabs, "Select Vehicle", 24.0, ColorRole::Primary);
            let v = button(tabs, "Vehicle", false, Act::Back);
            id(&mut tabs.commands(), v, "vehicle.tab.vehicle");
            let f = button(tabs, "Formation", true, Act::Noop);
            id(&mut tabs.commands(), f, "vehicle.tab.formation");
        });
        p.spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(16.0),
            width: Val::Percent(100.0),
            flex_grow: 1.0,
            min_height: Val::Px(0.0),
            ..default()
        })
        .with_children(|row| {
            let c1 = column(row, 30.0);
            row.commands()
                .entity(c1)
                .with_children(|c| catalogue_column(c, ed));
            let c2 = column(row, 40.0);
            row.commands()
                .entity(c2)
                .with_children(|c| train_column(c, ed));
            let c3 = column(row, 30.0);
            row.commands()
                .entity(c3)
                .with_children(|c| summary_column(c, ed, entries));
        });
        if !ed.status.is_empty() {
            label(p, ed.status.clone(), 13.0, ColorRole::Error);
        }
        p.spawn(Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::FlexEnd,
            column_gap: Val::Px(12.0),
            width: Val::Percent(100.0),
            ..default()
        })
        .with_children(|footer| {
            let back = button(footer, "Back", false, Act::Back);
            id(&mut footer.commands(), back, "formation.back");
            let reverse = button(footer, "Reverse train", false, Act::Reverse);
            id(&mut footer.commands(), reverse, "formation.reverse");
            let clear = button(footer, "Clear", false, Act::Clear);
            id(&mut footer.commands(), clear, "formation.clear");
            let save = button(footer, "Save", false, Act::Save);
            id(&mut footer.commands(), save, "formation.save");
            if ed.formation.is_empty() {
                footer.commands().entity(save).insert(Disabled);
            }
            let spawn = button(footer, "✓ Spawn", true, Act::Spawn);
            id(&mut footer.commands(), spawn, "formation.spawn");
            if !ed.can_spawn(entries.chosen) {
                footer.commands().entity(spawn).insert(Disabled);
            }
        });
    });
}
