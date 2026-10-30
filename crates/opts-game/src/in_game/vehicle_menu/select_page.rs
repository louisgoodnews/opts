//! Builder for the 3-column Select Vehicle modal.

use bevy::prelude::*;
use opts_gui::prelude::*;

use crate::new_game_screen::types::{GameCatalogue, VehicleItem};

use super::build::{button, column, label, scroll_list};
use super::entries::matches_search;
use super::{Action, MenuRoot, VehicleMenu};

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

/// Column 1: the vehicle list with its search field.
fn vehicle_column(row: &mut ChildSpawnerCommands, menu: &VehicleMenu, cat: &GameCatalogue) {
    let c1 = column(row, 30.0);
    row.commands().entity(c1).with_children(|c| {
        let shown: Vec<usize> = cat
            .vehicles
            .iter()
            .enumerate()
            .filter(|(_, v)| matches_search(v, &menu.search))
            .map(|(i, _)| i)
            .collect();
        label(
            c,
            if menu.search.is_empty() {
                format!("Vehicles ({})", cat.vehicles.len())
            } else {
                format!("Vehicles ({} of {})", shown.len(), cat.vehicles.len())
            },
            15.0,
            ColorRole::Primary,
        );
        // The search field: type to filter, Backspace deletes (Escape steps back).
        c.spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BorderRole(ColorRole::Outline),
            BackgroundRole(ColorRole::SurfaceContainerLowest),
            RadiusToken(RadiusLevel::Small),
        ))
        .with_children(|f| {
            let text = if menu.search.is_empty() {
                "Search: type to filter…".to_string()
            } else {
                format!("Search: {}|", menu.search)
            };
            label(f, text, 13.0, ColorRole::OnSurface);
        });
        scroll_list(c, 0, menu.scroll[0], Val::Auto, |list| {
            if shown.is_empty() {
                label(
                    list,
                    "No vehicle matches.",
                    12.0,
                    ColorRole::OnSurfaceVariant,
                );
            }
            for i in shown {
                let v = &cat.vehicles[i];
                let selected = menu.vehicle == Some(i);
                button(list, &v.full_name, selected, Action::PickVehicle(i));
            }
        });
    });
}

/// Column 3: the free entry points and the bus stops.
fn entry_column(
    row: &mut ChildSpawnerCommands,
    menu: &VehicleMenu,
    entries_resolved: (usize, bool),
) {
    let c3 = column(row, 30.0);
    row.commands().entity(c3).with_children(|c| {
        label(c, "Entry points", 15.0, ColorRole::Primary);
        if !entries_resolved.1 {
            label(
                c,
                format!(
                    "Locating entry points… {} of {} found; the others cannot be checked for blocking yet",
                    entries_resolved.0, menu.entry_total
                ),
                12.0,
                ColorRole::OnSurfaceVariant,
            );
        }
        scroll_list(c, 2, menu.scroll[2], Val::Auto, |list| {
            if menu.shown_entries.is_empty() {
                label(
                    list,
                    "No free entry point (all are blocked by vehicles).",
                    12.0,
                    ColorRole::OnSurfaceVariant,
                );
            }
            for (id, name) in &menu.shown_entries {
                let selected = menu.entry == Some(*id);
                button(list, name, selected, Action::PickEntry(*id));
            }
            // Bus stops: the vehicle stands with its front at the stop, if it is free.
            if !menu.shown_stops.is_empty() {
                label(list, "Bus stops", 15.0, ColorRole::Primary);
            }
            for (id, name) in &menu.shown_stops {
                let selected = menu.stop == Some(*id);
                button(list, name, selected, Action::PickStop(*id));
            }
        });
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

fn detail_column(c: &mut ChildSpawnerCommands, menu: &VehicleMenu, v: &VehicleItem) {
    label(c, v.full_name.clone(), 20.0, ColorRole::Primary);
    label(
        c,
        format!("{} · {}", v.manufacturer, v.model),
        12.0,
        ColorRole::OnSurfaceVariant,
    );
    // Only facts the content provides are shown (no "N/A" rows).
    if let Some(t) = v.mass_tonnes {
        label(
            c,
            format!("Empty weight: {t:.1} t"),
            12.0,
            ColorRole::OnSurface,
        );
    }
    scroll_list(c, 3, menu.scroll[3], Val::Px(110.0), |list| {
        label(list, v.description.clone(), 12.0, ColorRole::OnSurface);
    });
    label(c, "Repaint", 15.0, ColorRole::Primary);
    let names: Vec<String> = menu
        .repaints
        .as_ref()
        .map(|(_, r)| r.names.clone())
        .unwrap_or_default();
    scroll_list(c, 1, menu.scroll[1], Val::Auto, |list| {
        if names.is_empty() {
            label(
                list,
                "This vehicle has no repaints.",
                12.0,
                ColorRole::OnSurfaceVariant,
            );
        }
        for (i, name) in names.iter().enumerate() {
            let selected = if menu.repaint.is_empty() {
                i == 0
            } else {
                menu.repaint == *name
            };
            // The first (default) repaint is chosen with an empty name.
            let value = if i == 0 { String::new() } else { name.clone() };
            button(list, name, selected, Action::PickRepaint(value));
        }
    });
    button(c, "Configure vehicle…", false, Action::OpenConfig);
}
