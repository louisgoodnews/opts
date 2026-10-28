//! Builders of the menu pages.

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
                let reset = button(row, "Reset on road", false, Action::ResetOnRoad(*id));
                row.commands()
                    .entity(reset)
                    .insert(UiId::new(format!("vehicle.session.{id}.reset")));
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
        let change = button(p, "Change map…", false, Action::OpenChangeMap);
        p.commands()
            .entity(change)
            .insert(UiId::new("vehicle.change_map"));
        let setvars = button(p, "Setvars…", false, Action::OpenSetvars);
        p.commands()
            .entity(setvars)
            .insert(UiId::new("vehicle.setvars"));
        let configs = button(p, "Configurations…", false, Action::OpenConfigs);
        p.commands()
            .entity(configs)
            .insert(UiId::new("vehicle.configs"));
        let route = button(p, "Line and destination…", false, Action::OpenRoute);
        p.commands()
            .entity(route)
            .insert(UiId::new("vehicle.route"));
        button(p, "Close", false, Action::Close);
    });
}

pub(super) use super::select_page::build_select;

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
            setvar_rows(list, &items, false);
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

/// One row per option: the label with a stepper (choices and numbers) or a toggle (switches).
/// `live` makes the controls change the driven vehicle at once (the Setvars page) instead of the
/// choices of the vehicle that is about to be spawned.
pub(super) fn setvar_rows(
    list: &mut ChildSpawnerCommands,
    items: &[crate::new_game_screen::setvars_discovery::SetvarItem],
    live: bool,
) {
    for item in items {
        let step = |delta: i32| {
            if live {
                Action::LiveStep(item.name.clone(), delta)
            } else {
                Action::Step(item.name.clone(), delta)
            }
        };
        let toggle = if live {
            Action::LiveToggle(item.name.clone())
        } else {
            Action::Toggle(item.name.clone())
        };
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
                        button(stepper, "‹", false, step(-1));
                        label(stepper, item.value.to_string(), 12.0, ColorRole::OnSurface);
                        button(stepper, "›", false, step(1));
                    });
                }
                _ => {
                    let text = if item.value == 0 { "off (0)" } else { "on (1)" };
                    button(row, text, item.value != 0, toggle);
                }
            }
        });
    }
}
