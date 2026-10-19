//! Builders of the menu pages.

use super::entries::matches_search;
use super::*;

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

pub(super) fn button(
    p: &mut ChildSpawnerCommands,
    text: &str,
    selected: bool,
    action: Action,
) -> Entity {
    let builder = if selected {
        ButtonBuilder::filled(text)
    } else {
        ButtonBuilder::tonal(text)
    };
    let e = builder.spawn_child(p);
    p.commands().entity(e).insert(action);
    e
}

pub(super) fn scroll_list(
    p: &mut ChildSpawnerCommands,
    which: usize,
    offset: f32,
    height: Val,
    fill: impl FnOnce(&mut ChildSpawnerCommands),
) {
    p.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            width: Val::Percent(100.0),
            height,
            flex_grow: 1.0,
            min_height: Val::Px(0.0),
            overflow: Overflow::scroll_y(),
            padding: UiRect::right(Val::Px(6.0)),
            ..default()
        },
        ScrollPosition(Vec2::new(0.0, offset)),
        ListScroll(which),
    ))
    .with_children(fill);
}

pub(super) fn column(p: &mut ChildSpawnerCommands, width_percent: f32) -> Entity {
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

pub(super) fn build_main(
    commands: &mut Commands,
    _theme: &MaterialTheme,
    status: &str,
    rows: &[(RowSig, String)],
) {
    let (root, content) = ModalBuilder::new()
        .with_size_percent(40.0, 60.0)
        .with_width_range(420.0, 640.0)
        .spawn(commands);
    commands.entity(root).insert(MenuRoot);
    commands.entity(content).with_children(|p| {
        label(p, "Vehicle", 22.0, ColorRole::Primary);
        label(p, status, 13.0, ColorRole::OnSurfaceVariant);
        for ((id, active, _), text) in rows {
            p.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(6.0),
                width: Val::Percent(100.0),
                ..default()
            })
            .with_children(|row| {
                let switch = button(row, text, *active, Action::Switch(*id));
                row.commands()
                    .entity(switch)
                    .insert(UiId::new(format!("vehicle.session.{id}.switch")));
                let respawn = button(row, "Respawn", false, Action::Respawn(*id));
                row.commands()
                    .entity(respawn)
                    .insert(UiId::new(format!("vehicle.session.{id}.respawn")));
                let despawn = button(row, "Despawn", false, Action::DespawnId(*id));
                row.commands()
                    .entity(despawn)
                    .insert(UiId::new(format!("vehicle.session.{id}.despawn")));
            });
        }
        button(p, "Spawn vehicle…", true, Action::OpenSelect);
        let jump = button(p, "Jump to entry point…", false, Action::OpenJump);
        p.commands().entity(jump).insert(UiId::new("vehicle.jump"));
        let route = button(p, "Line and destination…", false, Action::OpenRoute);
        p.commands()
            .entity(route)
            .insert(UiId::new("vehicle.route"));
        button(p, "Close", false, Action::Close);
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
        p.spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(16.0),
            width: Val::Percent(100.0),
            flex_grow: 1.0,
            min_height: Val::Px(0.0),
            ..default()
        })
        .with_children(|row| {
            // Column 1: vehicles.
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
                        label(list, "No vehicle matches.", 12.0, ColorRole::OnSurfaceVariant);
                    }
                    for i in shown {
                        let v = &cat.vehicles[i];
                        let selected = menu.vehicle == Some(i);
                        button(list, &v.full_name, selected, Action::PickVehicle(i));
                    }
                });
            });
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
            // Column 3: free entry points.
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
                });
            });
        });
        if !menu.status.is_empty() {
            label(p, menu.status.clone(), 13.0, ColorRole::Error);
        }
        p.spawn(Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::FlexEnd,
            column_gap: Val::Px(12.0),
            width: Val::Percent(100.0),
            ..default()
        })
        .with_children(|footer| {
            button(footer, "Cancel", false, Action::Back);
            let ready = veh.is_some() && menu.entry.is_some();
            let confirm = button(footer, "✓ Confirm", true, Action::Confirm);
            if !ready {
                footer.commands().entity(confirm).insert(Disabled);
            }
        });
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

pub(super) fn build_config(
    commands: &mut Commands,
    _theme: &MaterialTheme,
    menu: &VehicleMenu,
    v: &VehicleItem,
) {
    let (root, content) = ModalBuilder::new()
        .with_size_percent(40.0, 70.0)
        .with_width_range(480.0, 720.0)
        .spawn(commands);
    commands.entity(root).insert(MenuRoot);
    commands.entity(root).insert(GlobalZIndex(1100));
    let items = get_vehicle_setvars(v, &menu.setvars, &menu.repaint);
    commands.entity(content).with_children(|p| {
        label(p, "Vehicle configuration", 20.0, ColorRole::Primary);
        label(p, v.full_name.clone(), 12.0, ColorRole::OnSurfaceVariant);
        if items.is_empty() {
            label(
                p,
                "This vehicle declares no configurable equipment options.",
                13.0,
                ColorRole::OnSurface,
            );
        }
        scroll_list(p, 4, menu.scroll[4], Val::Auto, |list| {
            for item in &items {
                list.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    width: Val::Percent(100.0),
                    ..default()
                })
                .with_children(|row| {
                    label(
                        row,
                        format!("{} ({})", item.label, item.name),
                        12.0,
                        ColorRole::OnSurface,
                    );
                    match item.kind {
                        SetvarKind::Choice | SetvarKind::Number => {
                            row.spawn(Node {
                                align_items: AlignItems::Center,
                                column_gap: Val::Px(6.0),
                                ..default()
                            })
                            .with_children(|stepper| {
                                button(stepper, "‹", false, Action::Step(item.name.clone(), -1));
                                label(stepper, item.value.to_string(), 12.0, ColorRole::OnSurface);
                                button(stepper, "›", false, Action::Step(item.name.clone(), 1));
                            });
                        }
                        _ => {
                            let text = if item.value == 0 { "off (0)" } else { "on (1)" };
                            button(
                                row,
                                text,
                                item.value != 0,
                                Action::Toggle(item.name.clone()),
                            );
                        }
                    }
                });
            }
        });
        p.spawn(Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::FlexEnd,
            column_gap: Val::Px(12.0),
            width: Val::Percent(100.0),
            ..default()
        })
        .with_children(|footer| {
            button(footer, "Reset", false, Action::ResetConfig);
            button(footer, "Done", true, Action::CloseConfig);
        });
    });
}
