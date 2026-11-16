//! UI builder for the route and timetable selection modal.

use super::super::build::{button, column, label, scroll_list};
use super::super::*;
use super::*;

mod columns;
use columns::{route_buttons, timetable_column};

/// Left: the depot's routes with the search field.
fn routes_column(row: &mut ChildSpawnerCommands, menu: &VehicleMenu, view: &RouteView) {
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
}

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
            routes_column(row, menu, view);
            timetable_column(row, menu, view);
        });
        route_buttons(p);
    });
}
