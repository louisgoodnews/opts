//! The route page: route and timetable pickers, jump to an entry.

pub(super) use super::build::{button, label, scroll_list};
use super::*;

/// Most depot routes the list builds buttons for; the search narrows the rest.
pub(super) const MAX_ROUTE_ROWS: usize = 300;

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

/// One line saying what is selected: the timetable run, the route's terminus, or free driving.
fn route_summary(
    handle: &opts_bevy::route::RouteHandle,
    schedule: &Option<opts_core::formats::timetable::ActiveSchedule>,
) -> String {
    match (schedule, handle.selection()) {
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
    }
}

/// The lines, tours or runs the timetable browser lists, by how far the user has drilled down.
fn timetable_view(
    menu: &VehicleMenu,
    tt: &opts_core::formats::timetable::Timetables,
) -> TimetableView {
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
    let summary = route_summary(handle, &schedule);
    let timetable = handle.timetables().map(|tt| timetable_view(menu, &tt));
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

mod build;
pub(super) use build::build_route;
mod jump;
pub(super) use jump::{build_jump, jump_to_entry};

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
