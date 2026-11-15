//! The timetables of a map: the parsed files and their indexes, and the schedule of one run.

use std::collections::HashMap;

use super::{StnLink, TtBusStop, TtLine, TtTrip};

/// Everything a map's `TTData` folder holds that buses use: the stop list, the line files with
/// their tours and the trip files.
#[derive(Debug, Clone, Default)]
pub struct Timetables {
    /// `Busstops.cfg`.
    pub stops: Vec<TtBusStop>,
    /// `(file stem, line file)` in name order.
    pub lines: Vec<(String, TtLine)>,
    pub(super) trips: HashMap<String, TtTrip>,
    pub(super) stop_by_id: HashMap<u32, usize>,
    /// `StnLinks.cfg`: the links between stations with their lengths.
    pub links: Vec<StnLink>,
    pub(super) link_by_pair: HashMap<(u32, u32), usize>,
}

/// One stop of a scheduled run, with its times as seconds of the day.
#[derive(Debug, Clone, PartialEq)]
pub struct ScheduledStop {
    /// Station id.
    pub station_id: u32,
    /// Name from `Busstops.cfg` (empty when the id is not in the list).
    pub name: String,
    /// Scheduled arrival (s after midnight; may exceed 86,400 for runs past midnight).
    pub arrival_s: f32,
    /// Scheduled departure (s after midnight).
    pub departure_s: f32,
    /// Distance along the run from its first stop (m), when `StnLinks.cfg` has a link for every
    /// pair of stations of the trip.
    pub distance_m: Option<f32>,
}

/// The schedule of one run of a tour: what `GetTT*` answer for a scheduled vehicle.
#[derive(Debug, Clone, PartialEq)]
pub struct ActiveSchedule {
    /// Line file stem the tour is in.
    pub line_file: String,
    /// Tour name.
    pub tour: String,
    /// Trip file stem.
    pub trip: String,
    /// Destination text of the trip.
    pub title: String,
    /// Line text of the trip.
    pub line: String,
    /// Start of the run (s after midnight).
    pub start_s: f32,
    /// The stops in running order.
    pub stops: Vec<ScheduledStop>,
}
