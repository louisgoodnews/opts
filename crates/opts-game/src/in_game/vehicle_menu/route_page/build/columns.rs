//! The right-hand timetable column and the footer buttons of the route page.

use super::super::super::build::{button, column, label, scroll_list};
use super::super::super::*;
use super::super::*;

/// Right: the map's timetables, by how far the user has drilled down.
pub(super) fn timetable_column(row: &mut ChildSpawnerCommands, menu: &VehicleMenu, view: &RouteView) {
    let right = column(row, 50.0);
    row.commands()
        .entity(right)
        .with_children(|c| match &view.timetable {
            None => label(
                c,
                "This map has no timetable files.",
                14.0,
                ColorRole::OnSurfaceVariant,
            ),
            Some(t) => {
                let (title, rows, nested): (String, Vec<(usize, String)>, u8) = match t {
                    TimetableView::Lines(v) => ("Timetables · line files".into(), v.clone(), 0),
                    TimetableView::Tours(n, v) => {
                        (format!("Timetables · {n} · tours"), v.clone(), 1)
                    }
                    TimetableView::Runs(n, v) => (format!("Timetables · {n} · runs"), v.clone(), 2),
                };
                label(c, title, 15.0, ColorRole::Primary);
                if nested > 0 {
                    let up = button(c, "◀ Up", false, Action::TtUp);
                    c.commands()
                        .entity(up)
                        .insert(UiId::new("vehicle.route.tt.up"));
                }
                scroll_list(c, 6, menu.scroll[6], Val::Percent(100.0), |list| {
                    for (i, text) in rows {
                        let action = match nested {
                            0 => Action::TtLine(i),
                            1 => Action::TtTour(i),
                            _ => Action::TtRun(i),
                        };
                        let b = button(list, &text, false, action);
                        list.commands()
                            .entity(b)
                            .insert(UiId::new(format!("vehicle.route.tt.{nested}.{i}")));
                    }
                });
            }
        });
}

/// *Cancel route* and *Back*.
pub(super) fn route_buttons(p: &mut ChildSpawnerCommands) {
    p.spawn(Node {
        flex_direction: FlexDirection::Row,
        column_gap: Val::Px(8.0),
        ..default()
    })
    .with_children(|b| {
        let clear = button(b, "Cancel route", false, Action::ClearRoute);
        b.commands()
            .entity(clear)
            .insert(UiId::new("vehicle.route.clear"));
        let back = button(b, "Back", true, Action::Back);
        b.commands()
            .entity(back)
            .insert(UiId::new("vehicle.route.back"));
    });
}
