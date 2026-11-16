//! Builder for the 3-column Select Vehicle modal.

use bevy::prelude::*;
use opts_gui::prelude::*;

use crate::new_game_screen::types::{GameCatalogue, VehicleItem};

use super::build::{button, column, label, scroll_list};
use super::entries::matches_search;
use super::{Action, MenuRoot, VehicleMenu};

mod entry_column;
mod vehicle_list;
use entry_column::entry_column;
use vehicle_list::{detail_column, vehicle_column};

/// The *Vehicle* and *Formation* tabs next to the title.
fn tab_row(p: &mut ChildSpawnerCommands) {
    p.spawn(Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: Val::Px(16.0),
        ..default()
    })
    .with_children(|tabs| {
        label(tabs, "Select Vehicle", 24.0, ColorRole::Primary);
        let vehicle_tab = button(tabs, "Vehicle", true, Action::OpenSelect);
        tabs.commands()
            .entity(vehicle_tab)
            .insert(UiId::new("vehicle.tab.vehicle"));
        let formation_tab = button(tabs, "Formation", false, Action::OpenFormation);
        tabs.commands()
            .entity(formation_tab)
            .insert(UiId::new("vehicle.tab.formation"));
    });
}

/// *Cancel* and *Confirm* (the latter only with a vehicle and a place to start).
fn confirm_row(p: &mut ChildSpawnerCommands, ready: bool) {
    p.spawn(Node {
        flex_direction: FlexDirection::Row,
        justify_content: JustifyContent::FlexEnd,
        column_gap: Val::Px(12.0),
        width: Val::Percent(100.0),
        ..default()
    })
    .with_children(|footer| {
        button(footer, "Cancel", false, Action::Back);
        let confirm = button(footer, "✓ Confirm", true, Action::Confirm);
        if !ready {
            footer.commands().entity(confirm).insert(Disabled);
        }
    });
}

pub(super) fn build_select(
    commands: &mut Commands,
    _theme: &MaterialTheme,
    menu: &VehicleMenu,
    cat: &GameCatalogue,
    entries_resolved: (usize, bool),
) {
    let (root, content) = ModalBuilder::new()
        .with_size_percent(94.0, 92.0)
        .with_width_range(900.0, 1700.0)
        .spawn(commands);
    commands.entity(root).insert(MenuRoot);
    let veh = menu.vehicle.and_then(|i| cat.vehicles.get(i));
    commands.entity(content).with_children(|p| {
        tab_row(p);
        p.spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(16.0),
            width: Val::Percent(100.0),
            flex_grow: 1.0,
            min_height: Val::Px(0.0),
            ..default()
        })
        .with_children(|row| {
            vehicle_column(row, menu, cat);
            // Column 2: description, repaints, configuration.
            let c2 = column(row, 40.0);
            row.commands().entity(c2).with_children(|c| {
                if let Some(v) = veh {
                    detail_column(c, menu, v);
                } else {
                    label(
                        c,
                        "No vehicle in the catalogue.",
                        14.0,
                        ColorRole::OnSurfaceVariant,
                    );
                }
            });
            entry_column(row, menu, entries_resolved);
        });
        if !menu.status.is_empty() {
            label(p, menu.status.clone(), 13.0, ColorRole::Error);
        }
        confirm_row(
            p,
            veh.is_some() && (menu.entry.is_some() || menu.stop.is_some()),
        );
    });
}
