//! The optional step *Trip* (M-0145): the timetable runs the player may drive with the chosen map,
//! bus and clock. Choosing one replaces the start point by the run's first stop; the drive then
//! follows that run.
//!
//! The runs come from the map's `TTData` (line files marked `[userallowed]`), read once per map
//! off the main thread. A run is offered when it departs within [`WINDOW_S`] after the chosen
//! clock on the chosen weekday and its first stop is a bus stop of `Busstops.cfg`.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::*;
use bevy::tasks::{futures::check_ready, IoTaskPool, Task};
use opts_core::formats::timetable::Timetables;
use opts_gui::patterns::{
    glyphs, spawn_master_detail, ListEntry, MasterDetail, MasterDetailConfig,
};
use opts_gui::prelude::*;

use super::planner::{PlannerState, PlannerStep};
use super::step_where::StepList;
use super::types::{GameCatalogue, NewGameSetup};
use super::ui::{column, keyed, keyed_args, plain};
use super::BuildCtx;
use crate::in_game::omsi_world::line_runs_on;

/// How long after the chosen clock a run may depart to be offered (s).
pub const WINDOW_S: u32 = 3_600;

/// Id of the *No trip* entry.
pub const NO_TRIP_ID: &str = "__no_trip__";

/// The trip chosen for the drive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TripChoice {
    /// Line file stem.
    pub line_file: String,
    /// Tour index in the line file.
    pub tour: usize,
    /// Run index in the tour.
    pub run: usize,
    /// Station id of the first stop (the bus stop object's ID code in the map).
    pub station_id: u32,
    /// Name of the first stop.
    pub stop_name: String,
    /// Line text.
    pub line: String,
    /// Destination text.
    pub destination: String,
    /// Departure from the first stop (s after midnight).
    pub depart_s: u32,
}

impl TripChoice {
    /// The list id: `line_file:tour:run`.
    #[must_use]
    pub fn id(&self) -> String {
        format!("{}:{}:{}", self.line_file, self.tour, self.run)
    }

    /// `HH:MM` of the departure.
    #[must_use]
    pub fn depart_text(&self) -> String {
        let m = (self.depart_s / 60) % (24 * 60);
        format!("{:02}:{:02}", m / 60, m % 60)
    }

    /// `92 → Stadtgrenze`; a trip without a line text (a depot run) is its destination alone.
    #[must_use]
    pub fn title(&self) -> String {
        if self.line.is_empty() {
            self.destination.clone()
        } else {
            format!("{} → {}", self.line, self.destination)
        }
    }
}

/// A run the step offers.
#[derive(Debug, Clone, PartialEq)]
pub struct TripOption {
    /// What choosing it sets.
    pub choice: TripChoice,
    /// The tour's name.
    pub tour_name: String,
    /// Name of the last stop.
    pub last_stop: String,
    /// Number of stops.
    pub stops: usize,
}

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

/// What the list was made for.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TripKey {
    map: String,
    clock_s: u32,
    weekday: u32,
}

/// The runs offered for the current choices.
#[derive(Resource, Default)]
pub struct TripState {
    key: Option<TripKey>,
    /// The runs, earliest first; empty when none fit (the step is then not offered).
    pub trips: Vec<TripOption>,
    /// The map's timetables are being read.
    pub loading: bool,
    timetables: HashMap<String, Option<Arc<Timetables>>>,
    task: Option<(String, Task<Option<Timetables>>)>,
}

impl TripState {
    /// The offered run with this list id.
    #[must_use]
    pub fn find(&self, id: &str) -> Option<&TripOption> {
        self.trips.iter().find(|t| t.choice.id() == id)
    }

    /// Timetables already read for a map (tests).
    pub fn insert_timetables(&mut self, map_id: &str, tt: Timetables) {
        self.timetables
            .insert(map_id.to_string(), Some(Arc::new(tt)));
    }
}

/// Whether the plan drives a bus: a road vehicle alone (no train, no drive without a vehicle).
fn drives_a_bus(setup: &NewGameSetup, catalogue: &GameCatalogue) -> bool {
    !setup.no_vehicle
        && setup.formation.is_empty()
        && catalogue
            .find_vehicle(&setup.selected_vehicle)
            .is_some_and(|v| v.class != "rail" && v.class != "aircraft")
}

/// ISO weekday of the plan's date (1 = Monday).
fn weekday_of(setup: &NewGameSetup) -> u32 {
    use chrono::Datelike;
    chrono::NaiveDate::from_ymd_opt(setup.year, setup.month, setup.day)
        .map_or(1, |d| d.weekday().number_from_monday())
}

/// Keeps the offered runs current: reads a map's timetables in the background, lists the runs
/// for map, bus, clock and weekday, drops a chosen trip that no longer fits, and leaves the step
/// when it is no longer offered.
pub fn refresh_trips(
    mut trips: ResMut<TripState>,
    mut setup: ResMut<NewGameSetup>,
    catalogue: Res<GameCatalogue>,
    mut state: ResMut<PlannerState>,
) {
    let mut arrived = false;
    if let Some((map, task)) = trips.task.as_mut() {
        if let Some(tt) = check_ready(task) {
            let map = map.clone();
            trips.timetables.insert(map, tt.map(Arc::new));
            trips.task = None;
            arrived = true;
        }
    }
    let map = catalogue
        .map_by_name(&setup.selected_map)
        .map(|m| m.id.clone());
    let key = map
        .filter(|_| drives_a_bus(&setup, &catalogue))
        .map(|map| TripKey {
            map,
            clock_s: setup.hour * 3_600 + setup.minute * 60,
            weekday: weekday_of(&setup),
        });
    if key == trips.key && !arrived {
        return;
    }
    trips.key.clone_from(&key);
    let list = match &key {
        None => Some(Vec::new()),
        Some(k) => match trips.timetables.get(&k.map) {
            Some(tt) => Some(
                tt.as_deref()
                    .map(|tt| list_trips(tt, k.clock_s, k.weekday))
                    .unwrap_or_default(),
            ),
            None => {
                if trips.task.as_ref().is_none_or(|(m, _)| *m != k.map) {
                    let map = k.map.clone();
                    let task = IoTaskPool::get().spawn(async move {
                        crate::omsi_root::known_omsi_root()
                            .map(|root| root.join("maps").join(&map))
                            .and_then(|d| opts_bevy::route::find_timetables(&d))
                    });
                    trips.task = Some((k.map.clone(), task));
                }
                None
            }
        },
    };
    let loading = list.is_none();
    if trips.loading != loading {
        trips.loading = loading;
    }
    let Some(list) = list else {
        // Asked again when the timetables arrive.
        trips.key = None;
        return;
    };
    let offered = !list.is_empty();
    if setup
        .trip
        .as_ref()
        .is_some_and(|c| !list.iter().any(|t| t.choice == *c))
    {
        setup.trip = None;
    }
    trips.trips = list;
    if state.trips_offered != offered {
        state.trips_offered = offered;
        state.built = None;
    }
    if !offered && state.step == PlannerStep::Trip {
        state.step = PlannerStep::Review;
    }
}

mod ui;
pub use ui::{fill_no_trip_detail, fill_trip_detail, spawn_trip};

#[cfg(test)]
mod tests;
