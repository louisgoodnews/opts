//! The selection as data: what the driver selected, why a selection fails, and the state behind the handle.

use super::*;

/// What the driver has selected, as the scripts and the protocol name it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteSelection {
    /// Route code as keyed on the IBIS: `line * 100 + course` (`13001`: line 130, course 01).
    pub route_code: i32,
    /// Line (`route_code / 100`).
    pub line: i32,
    /// Course (`route_code % 100`).
    pub course: i32,
    /// Destination code of the route (the terminus table's code column).
    pub terminus_code: i32,
    /// Line text of the route (`130`, `91.06A`; a depot may mark "no text" with `X`s).
    pub line_string: String,
    /// Route name of the depot (`URUH-NERV`).
    pub route_string: String,
}

/// Why a selection or a lookup failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteError {
    /// The vehicle has no depot file.
    NoDepot,
    /// The depot has no route with this code.
    UnknownRoute(i32),
    /// The vehicle's scripts have no IBIS keypad this layer knows (`ibis_eingabe`, `ibis_0`...).
    NoKeypad,
    /// The map has no timetable files.
    NoTimetables,
    /// The map's timetables have no such run (`line file/tour/run`).
    UnknownRun(String),
}

impl std::fmt::Display for RouteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoDepot => write!(f, "the vehicle has no depot (.hof) file"),
            Self::UnknownRoute(c) => write!(f, "the depot has no route {c}"),
            Self::NoKeypad => write!(f, "the vehicle's scripts have no IBIS keypad"),
            Self::NoTimetables => write!(f, "the map has no timetable files (TTData)"),
            Self::UnknownRun(r) => write!(f, "the timetables have no run {r}"),
        }
    }
}

impl std::error::Error for RouteError {}

/// The state behind a [`RouteHandle`].
#[derive(Debug, Default)]
pub struct RouteState {
    /// The depot the vehicle belongs to.
    pub hof: Option<Arc<HofFile>>,
    /// Index into the depot's routes of the selected route.
    pub active_route: Option<usize>,
    /// The stop the vehicle is at (index along the route).
    pub stop_index: usize,
    /// Delay against the timetable (s, late positive); free driving has none.
    pub delay_s: f32,
    /// The map's timetable files, if it has any.
    pub timetables: Option<Arc<Timetables>>,
    /// The run the driver follows, if a timetable is selected.
    pub schedule: Option<ActiveSchedule>,
}

/// Lower case without spaces and `*` (platform and padding marks): destination texts of the
/// timetable files and of the depot differ in both.
pub(super) fn flat(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_whitespace() && *c != '*')
        .collect::<String>()
        .to_lowercase()
}
