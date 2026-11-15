//! Which runs may appear now: their tour's rail vehicles, their phase and the stops on loaded track.

use super::super::*;
use crate::in_game::omsi_world::{run_phase, RunPhase};
use opts_core::formats::timetable::ActiveSchedule;

/// The group's rail vehicles of a run's tour; `None` when the run has no tour or the group has no
/// rail vehicle (remembered in `not_rail`).
pub(in super::super) fn rail_vehicles<'a>(
    pass: &Pass,
    pool: &'a RailAiPool,
    (run, schedule): (&RunRef, &ActiveSchedule),
    not_rail: &mut HashSet<String>,
) -> Option<Vec<&'a String>> {
    let tour = pass.tt.tour(&schedule.line_file, run.tour)?;
    let group = tour.vehicle_group.to_ascii_lowercase();
    if not_rail.contains(&group) {
        return None;
    }
    let rail: Vec<&String> = pool
        .group_vehicles
        .get(&group)
        .map(|v| {
            v.iter()
                .filter(|f| pool.rail_files.contains(&f.to_ascii_lowercase()))
                .collect()
        })
        .unwrap_or_default();
    if rail.is_empty() {
        not_rail.insert(group);
        return None;
    }
    Some(rail)
}

/// The stop the run is at or coming from and the part of the way to the next, when it is on the
/// rails now.
pub(in super::super) fn start_phase(
    schedule: &ActiveSchedule,
    now: f64,
) -> Option<(usize, Option<f64>)> {
    Some(match run_phase(schedule, now) {
        RunPhase::NotStarted { wait_s } if wait_s <= LEAD_S => (0, None),
        RunPhase::AtStop { index } => (index, None),
        RunPhase::Between { from, fraction } => (from, Some(fraction)),
        _ => return None,
    })
}

/// Where the run is: at its first stop, between two loaded stops, or on the way to a loaded stop
/// whose predecessor is not on loaded track (the train comes from upstream).
pub(in super::super) fn place_run(
    pass: &Pass,
    schedule: &ActiveSchedule,
    (first, offset): (usize, Option<f64>),
) -> Option<Layout> {
    let place_of = |k: usize| pass.place(schedule.stops.get(k)?.station_id);
    let upstream = matches!(run_phase(schedule, pass.now), RunPhase::Between { .. })
        && place_of(first).is_none()
        && place_of(first + 1).is_some();
    let from_stop = if upstream { first + 1 } else { first };
    let mut places: Places = Vec::new();
    for k in from_stop..schedule.stops.len() {
        match place_of(k) {
            Some(p) => places.push((k, p)),
            None => break,
        }
    }
    if places.is_empty() || (offset.is_some() && !upstream && places.len() < 2) {
        return None;
    }
    Some(Layout {
        places,
        upstream,
        from_stop,
        offset,
    })
}
