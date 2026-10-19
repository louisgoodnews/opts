//! The route page: route and timetable pickers, jump to an entry.

use super::build::{button, column, label, scroll_list};
use super::*;

/// Most depot routes the list builds buttons for; the search narrows the rest.
const MAX_ROUTE_ROWS: usize = 300;

/// What the route page shows: the depot's routes, the map's timetable lists and the selection.
pub(super) struct RouteView {
    pub(super) depot: Option<String>,
    /// `(route code, label)` of the routes matching the search.
    pub(super) routes: Vec<(i32, String)>,
    pub(super) total_routes: usize,
    pub(super) selected: Option<i32>,
    /// Text of the selection line.
    pub(super) summary: String,
    pub(super) timetable: Option<TimetableView>,
}

/// The right column: line files, or the tours of one, or the runs of a tour.
pub(super) enum TimetableView {
    Lines(Vec<(usize, String)>),
    Tours(String, Vec<(usize, String)>),
    Runs(String, Vec<(usize, String)>),
}

fn route_label(r: &opts_bevy::route::RouteSelection, terminus: &str) -> String {
    let line = if r.line_string.is_empty() {
        r.line.to_string()
    } else {
        r.line_string.clone()
    };
    format!("{line}/{:02}  {terminus}", r.course)
}

/// The labels of every route of the depot.
fn route_labels(handle: &opts_bevy::route::RouteHandle) -> Vec<(i32, String)> {
    handle
        .routes()
        .into_iter()
        .map(|r| {
            let terminus = handle.with(|s| {
                let hof = s.hof.as_ref()?;
                let (i, _) = hof.find_terminus_by_code(r.terminus_code)?;
                hof.terminus_string(i, 0).map(str::to_string)
            });
            let label = route_label(&r, terminus.as_deref().unwrap_or(""));
            (r.route_code, label)
        })
        .collect()
}

fn clock_text(s: f32) -> String {
    let t = (s.max(0.0).round() as u32) % 86_400;
    format!("{:02}:{:02}", t / 3600, t / 60 % 60)
}

pub(super) fn route_view(menu: &VehicleMenu, handle: &opts_bevy::route::RouteHandle) -> RouteView {
    let needle = menu.route_search.to_lowercase();
    let all = route_labels(handle);
    let total_routes = all.len();
    let routes: Vec<(i32, String)> = all
        .into_iter()
        .filter(|(_, l)| needle.is_empty() || l.to_lowercase().contains(&needle))
        .collect();
    let selected = handle.selection().map(|s| s.route_code);
    let schedule = handle.schedule();
    let summary = match (&schedule, handle.selection()) {
        (Some(sc), _) => format!(
            "Timetable: {} · {} {} → {}, start {}",
            sc.line_file,
            sc.tour,
            sc.line,
            sc.title,
            clock_text(sc.start_s)
        ),
        (None, Some(sel)) => {
            let terminus = route_labels(handle)
                .into_iter()
                .find(|(c, _)| *c == sel.route_code)
                .map(|(_, l)| l)
                .unwrap_or_default();
            format!("Route: {terminus}")
        }
        (None, None) => "No route selected (free driving).".to_string(),
    };
    let timetable = handle.timetables().map(|tt| {
        let line = menu
            .tt_line
            .and_then(|i| tt.lines.iter().filter(|(_, l)| l.user_allowed).nth(i));
        match (line, menu.tt_tour) {
            (None, _) => TimetableView::Lines(
                tt.lines
                    .iter()
                    .filter(|(_, l)| l.user_allowed)
                    .enumerate()
                    .map(|(i, (n, l))| (i, format!("{n}  ({} tours)", l.tours.len())))
                    .collect(),
            ),
            (Some((name, l)), None) => TimetableView::Tours(
                name.clone(),
                l.tours
                    .iter()
                    .enumerate()
                    .map(|(i, t)| {
                        (
                            i,
                            format!(
                                "{}  ·  {}  ({} runs)",
                                t.name,
                                t.vehicle_group,
                                t.trips.len()
                            ),
                        )
                    })
                    .collect(),
            ),
            (Some((name, l)), Some(ti)) => TimetableView::Runs(
                format!(
                    "{name} / {}",
                    l.tours.get(ti).map_or("", |t| t.name.as_str())
                ),
                l.tours
                    .get(ti)
                    .map(|t| {
                        t.trips
                            .iter()
                            .enumerate()
                            .map(|(i, r)| {
                                let trip = tt.trip(&r.trip);
                                (
                                    i,
                                    format!(
                                        "{}  {}  {}",
                                        clock_text(r.start_min * 60.0),
                                        trip.map_or("", |x| x.line.as_str()),
                                        trip.map_or(r.trip.as_str(), |x| x.title.as_str())
                                    ),
                                )
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            ),
        }
    });
    RouteView {
        depot: handle.with(|s| s.hof.as_ref().map(|h| h.name.clone())),
        routes,
        total_routes,
        selected,
        summary,
        timetable,
    }
}

fn set_status(world: &mut World, text: String) {
    if let Some(mut m) = world.get_resource_mut::<VehicleMenu>() {
        m.status = text;
    }
}

pub(super) fn pick_route(world: &mut World, code: i32) {
    let result =
        crate::control_route::set_route_impl(world, &serde_json::json!({ "route_code": code }));
    set_status(
        world,
        match result {
            Ok(v) if v["keypad"] == false => {
                "Route selected (this vehicle's IBIS is not supported: only its displays follow)."
                    .to_string()
            }
            Ok(_) => "Route selected; the IBIS is being keyed.".to_string(),
            Err(e) => format!("ERROR: {}", e.message),
        },
    );
}

pub(super) fn pick_route_text(world: &mut World, text: &str) {
    let needle = text.to_lowercase();
    let mut q =
        world.query_filtered::<&opts_bevy::route::VehicleRoute, With<opts_bevy::PlayerVehicle>>();
    let code = q.iter(world).next().and_then(|r| {
        route_labels(&r.handle)
            .into_iter()
            .find(|(_, l)| l.to_lowercase().contains(&needle))
            .map(|(c, _)| c)
    });
    match code {
        Some(c) => pick_route(world, c),
        None => set_status(world, format!("No route matches `{text}`.")),
    }
}

pub(super) fn pick_run(world: &mut World, line_index: usize, tour: usize, run: usize) {
    let mut q =
        world.query_filtered::<&opts_bevy::route::VehicleRoute, With<opts_bevy::PlayerVehicle>>();
    let line = q.iter(world).next().and_then(|r| {
        r.handle.timetables().and_then(|tt| {
            tt.lines
                .iter()
                .filter(|(_, l)| l.user_allowed)
                .nth(line_index)
                .map(|(n, _)| n.clone())
        })
    });
    match line {
        Some(l) => run_timetable(world, &l, tour, run),
        None => set_status(world, "That timetable line is gone.".to_string()),
    }
}

pub(super) fn run_timetable(world: &mut World, line: &str, tour: usize, run: usize) {
    let result = crate::control_route::set_timetable_impl(
        world,
        &serde_json::json!({ "line": line, "tour": tour, "run": run }),
    );
    set_status(
        world,
        match result {
            Ok(v) if v["route_code"].is_null() => {
                "Timetable followed; the depot has no route for its line and destination."
                    .to_string()
            }
            Ok(_) => "Timetable followed; the route is being keyed on the IBIS.".to_string(),
            Err(e) => format!("ERROR: {}", e.message),
        },
    );
}

pub(super) fn clear_route(world: &mut World) {
    let a = crate::control_route::set_route_impl(world, &serde_json::json!({ "clear": true }));
    let b = crate::control_route::set_timetable_impl(world, &serde_json::json!({ "clear": true }));
    set_status(
        world,
        match a.and(b) {
            Ok(_) => "Route and timetable cancelled.".to_string(),
            Err(e) => format!("ERROR: {}", e.message),
        },
    );
}

mod jump;
pub(super) use jump::{build_jump, jump_to_entry};

pub(super) fn build_route(
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

/// Mouse wheel over a list scrolls it.
pub(super) fn scroll_lists(
    mut wheel: MessageReader<MouseWheel>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut lists: Query<(&mut ScrollPosition, &UiGlobalTransform, &ComputedNode), With<ListScroll>>,
) {
    let Ok(window) = window.single() else {
        return;
    };
    let Some(cursor) = window.physical_cursor_position() else {
        wheel.clear();
        return;
    };
    for e in wheel.read() {
        let delta = -e.y * 36.0;
        for (mut pos, transform, node) in &mut lists {
            if !node.contains_point(*transform, cursor) {
                continue;
            }
            let max =
                ((node.content_size().y - node.size().y) * node.inverse_scale_factor()).max(0.0);
            pos.y = (pos.y + delta).clamp(0.0, max);
            break;
        }
    }
}
