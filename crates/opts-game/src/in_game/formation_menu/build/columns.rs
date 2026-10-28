//! Train and summary columns of the formation editor modal.

use super::*;

pub(super) fn train_column(c: &mut ChildSpawnerCommands, ed: &FormationEditor) {
    c.spawn(Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: Val::Px(10.0),
        ..default()
    })
    .with_children(|head| {
        label(
            head,
            if ed.renaming {
                format!("Name: {}|", ed.formation.name)
            } else {
                format!("Train: {}", ed.formation.name)
            },
            18.0,
            ColorRole::Primary,
        );
        let rename = button(
            head,
            if ed.renaming { "Done" } else { "Rename" },
            ed.renaming,
            Act::Rename,
        );
        id(&mut head.commands(), rename, "formation.rename");
    });
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

pub(super) fn summary_column(
    c: &mut ChildSpawnerCommands,
    ed: &FormationEditor,
    entries: &Entries,
) {
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
