//! The widgets of the formation editor page.

use bevy::prelude::*;
use opts_gui::prelude::*;

use super::{Act, FormationEditor, FormationList, FormationRoot, ListKind};

fn label(p: &mut ChildSpawnerCommands, text: impl Into<String>, size: f32, role: ColorRole) {
    p.spawn((
        Text::new(text.into()),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        ForegroundRole(role),
    ));
}

fn button(p: &mut ChildSpawnerCommands, text: &str, selected: bool, act: Act) -> Entity {
    let builder = if selected {
        ButtonBuilder::filled(text)
    } else {
        ButtonBuilder::tonal(text)
    };
    let e = builder.spawn_child(p);
    p.commands().entity(e).insert(act);
    e
}

fn scroll_list(
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

fn id(commands: &mut Commands, e: Entity, name: &str) {
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
            let spawn = button(footer, "✓ Spawn", true, Act::Spawn);
            id(&mut footer.commands(), spawn, "formation.spawn");
            if !ed.can_spawn(entries.chosen) {
                footer.commands().entity(spawn).insert(Disabled);
            }
        });
    });
}

fn catalogue_column(c: &mut ChildSpawnerCommands, ed: &FormationEditor) {
    c.spawn(Node {
        flex_direction: FlexDirection::Row,
        column_gap: Val::Px(8.0),
        ..default()
    })
    .with_children(|tabs| {
        let cars = button(
            tabs,
            "Rail cars",
            ed.list == ListKind::Cars,
            Act::List(ListKind::Cars),
        );
        id(&mut tabs.commands(), cars, "formation.list.cars");
        let presets = button(
            tabs,
            "Trains",
            ed.list == ListKind::Presets,
            Act::List(ListKind::Presets),
        );
        id(&mut tabs.commands(), presets, "formation.list.presets");
    });
    let shown = ed.shown_list();
    label(
        c,
        match ed.list {
            ListKind::Cars => format!("Rail vehicles ({} of {})", shown.len(), ed.cars.len()),
            ListKind::Presets => format!("Consists ({} of {})", shown.len(), ed.presets.len()),
        },
        15.0,
        ColorRole::Primary,
    );
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
        let text = if ed.search.is_empty() {
            "Search: type to filter…".to_string()
        } else {
            format!("Search: {}|", ed.search)
        };
        label(f, text, 13.0, ColorRole::OnSurface);
    });
    scroll_list(c, 0, ed.scroll[0], |list| {
        if shown.is_empty() {
            label(list, "Nothing matches.", 12.0, ColorRole::OnSurfaceVariant);
        }
        for (i, text) in shown {
            let (act, selected) = match ed.list {
                ListKind::Cars => (Act::Pick(i), ed.pick == Some(i)),
                ListKind::Presets => (Act::PickPreset(i), false),
            };
            button(list, &text, selected, act);
        }
    });
    if ed.list == ListKind::Cars {
        let add = button(c, "Add to train ▶", true, Act::Add);
        id(&mut c.commands(), add, "formation.add");
        if ed.pick.is_none() {
            c.commands().entity(add).insert(Disabled);
        }
    }
}

fn train_column(c: &mut ChildSpawnerCommands, ed: &FormationEditor) {
    label(
        c,
        format!("Train: {}", ed.formation.name),
        18.0,
        ColorRole::Primary,
    );
    label(
        c,
        match &ed.formation.source {
            opts_core::formats::trains::FormationSource::Custom => "Composed here".to_owned(),
            opts_core::formats::trains::FormationSource::ZugFile(p) => format!("From {p}"),
        },
        12.0,
        ColorRole::OnSurfaceVariant,
    );
    scroll_list(c, 1, ed.scroll[1], |list| {
        if ed.formation.is_empty() {
            label(
                list,
                "The train is empty. Pick a rail car and press Add, or choose a consist under Trains.",
                12.0,
                ColorRole::OnSurfaceVariant,
            );
        }
        for (i, text) in ed.row_texts().into_iter().enumerate() {
            list.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(6.0),
                width: Val::Percent(100.0),
                ..default()
            })
            .with_children(|row| {
                let main = button(row, &text, ed.selected == Some(i), Act::Row(i));
                id(&mut row.commands(), main, &format!("formation.car.{i}"));
                for (t, act, name) in [
                    ("Up", Act::Up(i), "up"),
                    ("Down", Act::Down(i), "down"),
                    ("Turn", Act::Turn(i), "turn"),
                    ("Pulls", Act::Power(i), "power"),
                    ("✕", Act::Remove(i), "remove"),
                ] {
                    let b = button(row, t, false, act);
                    id(&mut row.commands(), b, &format!("formation.car.{i}.{name}"));
                }
            });
        }
    });
}

fn summary_column(c: &mut ChildSpawnerCommands, ed: &FormationEditor, entries: &Entries) {
    label(c, "Summary", 15.0, ColorRole::Primary);
    label(c, ed.summary_text(), 13.0, ColorRole::OnSurface);
    if ed.issues.is_empty() && !ed.formation.is_empty() {
        label(c, "No problems found.", 12.0, ColorRole::OnSurfaceVariant);
    }
    for issue in &ed.issues {
        label(c, format!("• {issue}"), 12.0, ColorRole::Error);
    }
    // A disabled Spawn says why.
    if ed.issues.is_empty() && ed.formation.len() >= 2 && entries.chosen.is_none() {
        label(
            c,
            "• choose an entry point to spawn",
            12.0,
            ColorRole::Error,
        );
    }
    label(c, "Entry points", 15.0, ColorRole::Primary);
    if !entries.progress.1 {
        label(
            c,
            format!("Locating entry points… {} found", entries.progress.0),
            12.0,
            ColorRole::OnSurfaceVariant,
        );
    }
    scroll_list(c, 2, ed.scroll[2], |list| {
        if entries.free.is_empty() {
            label(
                list,
                "No free entry point (all are blocked by vehicles).",
                12.0,
                ColorRole::OnSurfaceVariant,
            );
        }
        for (eid, name) in entries.free {
            button(list, name, entries.chosen == Some(*eid), Act::Entry(*eid));
        }
    });
    label(
        c,
        "The train is put on the nearest track within 150 m of the entry point.",
        11.0,
        ColorRole::OnSurfaceVariant,
    );
}
