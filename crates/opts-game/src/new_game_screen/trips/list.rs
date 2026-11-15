//! Which timetable runs the player may drive: the weekday rule of a tour and the window of runs.

use super::*;

/// Whether a tour runs on ISO weekday `weekday`: the line file's days (`line_runs_on`) and the
/// days its own name gives in short form (`Mo-Fr 6`, `Sa 4`, `So 2`, `Sa+So`), as Berlin-Spandau
/// names them in a line file without days. A name without days runs on every day of its file.
#[must_use]
pub fn tour_runs_on(name: &str, weekday: u32) -> bool {
    let n = name.to_lowercase();
    let mut days = Vec::new();
    for word in n.split(|c: char| !c.is_alphanumeric() && c != '-') {
        match word {
            "mo-fr" => days.extend(1..=5),
            "mo-do" => days.extend(1..=4),
            "mo-sa" => days.extend(1..=6),
            "fr" => days.push(5),
            "sa" => days.push(6),
            "so" => days.push(7),
            _ => {}
        }
    }
    if days.is_empty() {
        line_runs_on(&n, weekday)
    } else {
        days.contains(&weekday)
    }
}

/// The runs of `tt` the player may drive that depart within [`WINDOW_S`] after `clock_s` on ISO
/// weekday `weekday` (1 = Monday; the days of the line file and the tour), earliest first. A run starting after midnight counts for a
/// clock late in the evening.
#[must_use]
pub fn list_trips(tt: &Timetables, clock_s: u32, weekday: u32) -> Vec<TripOption> {
    let mut out = Vec::new();
    for (stem, line) in &tt.lines {
        if !line.user_allowed || !line_runs_on(stem, weekday) {
            continue;
        }
        for (t, tour) in line.tours.iter().enumerate() {
            if !tour_runs_on(&tour.name, weekday) {
                continue;
            }
            for (r, run) in tour.trips.iter().enumerate() {
                let start = (run.start_min.max(0.0) * 60.0).round() as u32;
                let start = if start < clock_s {
                    start + 86_400
                } else {
                    start
                };
                if start - clock_s > WINDOW_S {
                    continue;
                }
                let Some(schedule) = tt.schedule(stem, t, r) else {
                    continue;
                };
                let (Some(first), Some(last)) = (schedule.stops.first(), schedule.stops.last())
                else {
                    continue;
                };
                if schedule.stops.len() < 2 || tt.stop(first.station_id).is_none() {
                    continue;
                }
                out.push(TripOption {
                    choice: TripChoice {
                        line_file: stem.clone(),
                        tour: t,
                        run: r,
                        station_id: first.station_id,
                        stop_name: first.name.clone(),
                        line: schedule.line.trim().to_string(),
                        destination: schedule.title.trim().to_string(),
                        depart_s: first.departure_s.max(0.0).round() as u32,
                    },
                    tour_name: tour.name.clone(),
                    last_stop: last.name.clone(),
                    stops: schedule.stops.len(),
                });
            }
        }
    }
    out.sort_by(|a, b| {
        (a.choice.depart_s, &a.choice.line, &a.choice.destination).cmp(&(
            b.choice.depart_s,
            &b.choice.line,
            &b.choice.destination,
        ))
    });
    out
}
