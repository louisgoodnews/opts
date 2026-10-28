//! UI builder for the route and timetable selection modal.

use super::super::build::{button, column, label, scroll_list};
use super::super::*;
use super::*;

pub fn build_route(
    commands: &mut Commands,
    _theme: &MaterialTheme,
    menu: &VehicleMenu,
    view: Option<&RouteView>,
) {
    let (root, content) = ModalBuilder::new()
        .with_size_percent(80.0, 88.0)
        .with_width_range(760.0, 1400.0)
        .spawn(commands);
    commands.entity(root).insert(MenuRoot);
    commands.entity(content).with_children(|p| {
        label(p, "Line and destination", 24.0, ColorRole::Primary);
        let Some(view) = view else {
            label(
                p,
                "There is no player vehicle with a route.",
                14.0,
                ColorRole::OnSurfaceVariant,
            );
            let back = button(p, "Back", false, Action::Back);
            p.commands()
                .entity(back)
                .insert(UiId::new("vehicle.route.back"));
            return;
        };
        label(p, view.summary.clone(), 14.0, ColorRole::OnSurface);
        if !menu.status.is_empty() {
            label(p, menu.status.clone(), 13.0, ColorRole::OnSurfaceVariant);
        }
        p.spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(16.0),
            width: Val::Percent(100.0),
            flex_grow: 1.0,
            min_height: Val::Px(0.0),
            ..default()
        })
        .with_children(|row| {
            // Left: the depot's routes.
            let left = column(row, 50.0);
            row.commands().entity(left).with_children(|c| {
                label(
                    c,
                    format!(
                        "Depot routes{} ({} of {})",
                        view.depot
                            .as_deref()
                            .map_or(String::new(), |d| format!(" · {d}")),
                        view.routes.len(),
                        view.total_routes
                    ),
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
                    let text = if menu.route_search.is_empty() {
                        "Search: type a line or destination…".to_string()
                    } else {
                        format!("Search: {}|", menu.route_search)
                    };
                    label(f, text, 13.0, ColorRole::OnSurface);
                });
                scroll_list(c, 5, menu.scroll[5], Val::Percent(100.0), |list| {
                    for (code, text) in view.routes.iter().take(MAX_ROUTE_ROWS) {
                        let b = button(
                            list,
                            text,
                            view.selected == Some(*code),
                            Action::PickRoute(*code),
                        );
                        list.commands()
                            .entity(b)
                            .insert(UiId::new(format!("vehicle.route.pick.{code}")));
                    }
                    if view.routes.len() > MAX_ROUTE_ROWS {
                        label(
                            list,
                            format!(
                                "… {} more: type to narrow the list",
                                view.routes.len() - MAX_ROUTE_ROWS
                            ),
                            13.0,
                            ColorRole::OnSurfaceVariant,
                        );
                    }
                });
            });
            // Right: the map's timetables.
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
                            TimetableView::Lines(v) => {
                                ("Timetables · line files".into(), v.clone(), 0)
                            }
                            TimetableView::Tours(n, v) => {
                                (format!("Timetables · {n} · tours"), v.clone(), 1)
                            }
                            TimetableView::Runs(n, v) => {
                                (format!("Timetables · {n} · runs"), v.clone(), 2)
                            }
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
        });
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
    });
}
