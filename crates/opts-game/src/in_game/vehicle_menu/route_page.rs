//! The route page: route and timetable pickers, jump to an entry.

pub(super) use super::build::{button, label, scroll_list};
use super::*;

mod labels;
use labels::{clock_text, route_labels, route_summary};
mod timetable;
use timetable::timetable_view;
mod pick;
use pick::set_status;
pub(super) use pick::{clear_route, pick_route, pick_route_text, pick_run, run_timetable};

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
