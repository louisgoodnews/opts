//! The timetable browser: the lines, tours or runs of the map, by how far the user has drilled down.

use super::*;

/// The lines, tours or runs the timetable browser lists, by how far the user has drilled down.
pub(super) fn timetable_view(
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
