//! The catalogue column: the entries of the list the chosen tab shows.

use super::*;

pub(super) fn catalogue_column(c: &mut ChildSpawnerCommands, ed: &FormationEditor) {
    list_tabs(c, ed);
    let shown = ed.shown_list();
    label(
        c,
        match ed.list {
            ListKind::Cars => format!("Rail vehicles ({} of {})", shown.len(), ed.cars.len()),
            ListKind::Presets => format!("Consists ({} of {})", shown.len(), ed.presets.len()),
            ListKind::Saved => format!("Saved trains ({} of {})", shown.len(), ed.saved.len()),
        },
        15.0,
        ColorRole::Primary,
    );
    search_field(c, ed);
    scroll_list(c, 0, ed.scroll[0], |list| {
        if shown.is_empty() {
            label(list, "Nothing matches.", 12.0, ColorRole::OnSurfaceVariant);
        }
        for (i, text) in shown {
            let (act, selected) = match ed.list {
                ListKind::Cars => (Act::Pick(i), ed.pick == Some(i)),
                ListKind::Presets => (Act::PickPreset(i), false),
                ListKind::Saved => (
                    Act::PickSaved(i),
                    ed.saved_id.as_deref() == ed.saved.get(i).map(|s| s.0.as_str()),
                ),
            };
            if ed.list == ListKind::Saved {
                list.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(6.0),
                    width: Val::Percent(100.0),
                    ..default()
                })
                .with_children(|row| {
                    let load = button(row, &text, selected, act);
                    id(&mut row.commands(), load, &format!("formation.saved.{i}"));
                    let del = button(row, "✕", false, Act::DeleteSaved(i));
                    id(
                        &mut row.commands(),
                        del,
                        &format!("formation.saved.{i}.delete"),
                    );
                });
            } else {
                button(list, &text, selected, act);
            }
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
