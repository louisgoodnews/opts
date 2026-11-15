//! The configurations page's view: one row's widgets and the page itself.

use super::super::build::{button, label, scroll_list};
use super::super::*;
use super::rows::{ConfigRow, ConfigsView};
use crate::new_game_screen::thumbnails::spawn_config_thumb;

/// One saved configuration: name (or the rename field), detail, notices and its buttons.
fn config_row(list: &mut ChildSpawnerCommands, view: &ConfigsView, row: &ConfigRow) {
    let line = list
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(10.0),
            align_items: AlignItems::Center,
            width: Val::Percent(100.0),
            ..default()
        })
        .id();
    let thumb = spawn_config_thumb(&mut list.commands(), &row.vehicle_id, &row.class);
    list.commands().entity(line).add_child(thumb);
    list.commands().entity(line).with_children(|line| {
        line.spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(2.0),
            flex_grow: 1.0,
            min_width: Val::Px(0.0),
            ..default()
        })
        .with_children(|r| {
            let renaming = view.renaming.as_ref().filter(|(id, _)| *id == row.id);
            match renaming {
                Some((_, text)) => {
                    label(r, format!("Name: {text}|"), 15.0, ColorRole::Primary);
                }
                None => label(r, row.name.clone(), 15.0, ColorRole::Primary),
            }
            label(r, row.detail.clone(), 12.0, ColorRole::OnSurfaceVariant);
            if !row.blocked.is_empty() {
                label(r, row.blocked.clone(), 12.0, ColorRole::Error);
            } else if !row.notice.is_empty() {
                label(r, row.notice.clone(), 12.0, ColorRole::OnSurfaceVariant);
            }
            config_buttons(r, view, row, renaming.is_some());
        });
    });
}

/// The buttons of a row: rename field's *Save name*/*Cancel*, or *Spawn*, *Rename*, *Delete*.
fn config_buttons(
    r: &mut ChildSpawnerCommands,
    view: &ConfigsView,
    row: &ConfigRow,
    renaming: bool,
) {
    r.spawn(Node {
        flex_direction: FlexDirection::Row,
        column_gap: Val::Px(6.0),
        ..default()
    })
    .with_children(|buttons| {
        if renaming {
            let ok = button(buttons, "Save name", true, Action::CommitRename);
            buttons
                .commands()
                .entity(ok)
                .insert(UiId::new("vehicle.config.rename.save"));
            button(buttons, "Cancel", false, Action::CancelRename);
            return;
        }
        if row.blocked.is_empty() {
            let b = button(buttons, "Spawn", true, Action::SpawnConfig(row.id.clone()));
            buttons
                .commands()
                .entity(b)
                .insert(UiId::new(format!("vehicle.config.{}.spawn", row.id)));
        }
        let rename = button(
            buttons,
            "Rename",
            false,
            Action::RenameConfig(row.id.clone()),
        );
        buttons
            .commands()
            .entity(rename)
            .insert(UiId::new(format!("vehicle.config.{}.rename", row.id)));
        let asked = view.confirm_delete.as_deref() == Some(row.id.as_str());
        let text = if asked {
            "Delete? Press again"
        } else {
            "Delete"
        };
        let b = button(buttons, text, asked, Action::DeleteConfig(row.id.clone()));
        buttons
            .commands()
            .entity(b)
            .insert(UiId::new(format!("vehicle.config.{}.delete", row.id)));
    });
}

/// The page.
pub(in super::super) fn build_configs(commands: &mut Commands, menu: &VehicleMenu, can_save: bool) {
    let (root, content) = ModalBuilder::new()
        .with_size_percent(44.0, 80.0)
        .with_width_range(460.0, 700.0)
        .spawn(commands);
    commands.entity(root).insert(MenuRoot);
    let view = &menu.configs;
    commands.entity(content).with_children(|p| {
        label(p, "Configurations", 22.0, ColorRole::Primary);
        label(
            p,
            "A saved bus with its repaint and options. Spawn builds it as a new vehicle at the start point.",
            13.0,
            ColorRole::OnSurfaceVariant,
        );
        if let Some(problem) = &view.problem {
            label(p, problem.clone(), 13.0, ColorRole::Error);
        }
        if !menu.status.is_empty() {
            label(p, menu.status.clone(), 13.0, ColorRole::OnSurfaceVariant);
        }
        if view.rows.is_empty() && view.problem.is_none() {
            label(p, "No configuration is saved yet.", 13.0, ColorRole::OnSurfaceVariant);
        }
        scroll_list(p, 2, menu.scroll[2], Val::Percent(100.0), |list| {
            for row in &view.rows {
                config_row(list, view, row);
            }
        });
        if can_save {
            let save = button(p, "Save current vehicle", false, Action::SaveConfig);
            p.commands().entity(save).insert(UiId::new("vehicle.config.save"));
        }
        button(p, "Back", false, Action::Back);
    });
}
