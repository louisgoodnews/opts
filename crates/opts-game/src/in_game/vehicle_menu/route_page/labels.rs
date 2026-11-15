//! Labels of the depot routes and the one-line summary of the selection.

use super::*;

pub(super) fn route_label(r: &opts_bevy::route::RouteSelection, terminus: &str) -> String {
    let line = if r.line_string.is_empty() {
        r.line.to_string()
    } else {
        r.line_string.clone()
    };
    format!("{line}/{:02}  {terminus}", r.course)
}

/// The labels of every route of the depot.
pub(super) fn route_labels(handle: &opts_bevy::route::RouteHandle) -> Vec<(i32, String)> {
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

pub(super) fn clock_text(s: f32) -> String {
    let t = (s.max(0.0).round() as u32) % 86_400;
    format!("{:02}:{:02}", t / 3600, t / 60 % 60)
}

/// One line saying what is selected: the timetable run, the route's terminus, or free driving.
pub(super) fn route_summary(
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
