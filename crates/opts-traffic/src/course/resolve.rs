//! The course of a trip as paths of the map: built from the timetable files, resolved against the
//! lane graph by [`Lane::key`](crate::graph::Lane).
//!
//! Field meanings are the open question Q-T04 (`docs/src/formats/map/timetables.md`): the first
//! value of an entry is the element's ID code, the second the direction (taken here as `1` =
//! against the path, anything else = along it, ❓), the third the path number (`number`). A
//! [`CourseLane`] keeps the three values; nothing is guessed beyond that mapping, and the corpus
//! check (milestone M-0105, step 1) decides whether it holds.

use std::collections::HashMap;

use opts_core::formats::timetable::{StnLinkEntry, Timetables, TtTrackEntry, TtTrip};

use crate::graph::{LaneGraph, LaneId};

/// One path of a course: the element's ID code, the path number and the travel direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CourseLane {
    /// ID code of the spline or object the path belongs to (the lane's `key.0`).
    pub source: u64,
    /// Path number within that element (matched against the lane's `key.1`).
    pub number: u32,
    /// Travelled against the path's own direction.
    pub reversed: bool,
}

/// Something in a trip that could not be turned into course lanes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CourseGap {
    /// No station link (`StnLinks.cfg`) joins two consecutive stations of a Type 2 trip.
    MissingLink {
        /// Station the link starts at.
        from: u32,
        /// Station the link ends at.
        to: u32,
    },
    /// A Type 1 trip names a track file that is not in the folder.
    MissingTrack {
        /// The name the trip gives.
        name: String,
    },
}

/// The course as the timetables state it, before the graph is consulted.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CourseDraft {
    /// The paths in running order.
    pub lanes: Vec<CourseLane>,
    /// The stations in running order, each with the number of course paths before it.
    pub stops: Vec<(usize, u32)>,
    /// What could not be read; the course is incomplete when this is not empty.
    pub gaps: Vec<CourseGap>,
}

/// Entries of a trip into [`CourseLane`]s, in running order.
///
/// A Type 2 trip (stations are `[station_typ2]` stop IDs) takes the `StnLinks.cfg` link of each
/// consecutive pair of stations; a link stored the other way round is walked backwards. A Type 1
/// trip (its `[station]` records are legacy) takes the entries of its `.ttr` track file.
#[must_use]
pub fn course_from_trip(timetables: &Timetables, trip: &TtTrip) -> CourseDraft {
    let mut draft = CourseDraft::default();
    let rail = trip.stations.iter().any(|s| s.legacy);
    if rail {
        match timetables.track(&trip.name) {
            Some(entries) => {
                draft.lanes.extend(entries.iter().map(track_lane));
                if let Some(first) = trip.stations.first() {
                    draft.stops.push((0, first.id));
                }
            }
            None => draft.gaps.push(CourseGap::MissingTrack {
                name: trip.name.clone(),
            }),
        }
        return draft;
    }
    for (i, station) in trip.stations.iter().enumerate() {
        if i == 0 {
            draft.stops.push((0, station.id));
            continue;
        }
        let from = trip.stations[i - 1].id;
        let to = station.id;
        match timetables.link(from, to) {
            Some(link) => {
                let walks_forward = link.from_id == from && link.to_id == to;
                if walks_forward {
                    draft
                        .lanes
                        .extend(link.entries.iter().map(|e| entry_lane(e, false)));
                } else {
                    // Stored as `to -> from`: the pieces in reverse order, each travelled backwards.
                    draft
                        .lanes
                        .extend(link.entries.iter().rev().map(|e| entry_lane(e, true)));
                }
            }
            None => draft.gaps.push(CourseGap::MissingLink { from, to }),
        }
        draft.stops.push((draft.lanes.len(), to));
    }
    draft
}

/// A `[StnLink_entry]` as a course path. The second value `1` is taken as travel against the
/// path; the reversed walk of a link flips it.
fn entry_lane(e: &StnLinkEntry, flip: bool) -> CourseLane {
    CourseLane {
        source: u64::from(e.id),
        number: e.c.max(0) as u32,
        reversed: (e.b == 1) != flip,
    }
}

/// A `[track_entry]` as a course path (first value after the ID: the direction; second: the
/// path number).
fn track_lane(e: &TtTrackEntry) -> CourseLane {
    CourseLane {
        source: u64::from(e.id),
        number: e.values[1].max(0.0) as u32,
        reversed: e.values[0] == 1.0,
    }
}

/// One course path found in the graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedLane {
    /// Index into [`CourseDraft::lanes`] (the course's running order).
    pub course_index: usize,
    /// The graph lane with `Lane.key == (source, number)`.
    pub lane: LaneId,
}

/// The course looked up in a lane graph. Nothing is dropped: entries without a lane are in
/// `unresolved`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Resolution {
    /// The entries that were found, in running order.
    pub resolved: Vec<ResolvedLane>,
    /// `(index in the course, entry)` of every entry no lane has the key of.
    pub unresolved: Vec<(usize, CourseLane)>,
    /// Course indices of entries whose lane is not a successor of the lane before it, where both
    /// lanes resolved and the entries are adjacent in the course (hypothesis H1 of the timetable
    /// page: a course is a chain of successors).
    pub broken_joins: Vec<usize>,
}

impl Resolution {
    /// Share of the course's entries that resolved (`1.0` for an empty course).
    #[must_use]
    pub fn resolved_fraction(&self) -> f64 {
        let total = self.resolved.len() + self.unresolved.len();
        if total == 0 {
            1.0
        } else {
            self.resolved.len() as f64 / total as f64
        }
    }
}

impl LaneGraph {
    /// Looks each entry of `course` up by [`Lane::key`](crate::graph::Lane) and checks the joins
    /// between consecutive entries that were found.
    #[must_use]
    pub fn resolve_course(&self, course: &[CourseLane]) -> Resolution {
        let by_key: HashMap<(u64, u32), LaneId> = self
            .lane_ids()
            .filter_map(|id| self.lane(id).map(|l| (l.key, id)))
            .collect();
        let mut out = Resolution::default();
        for (index, entry) in course.iter().enumerate() {
            match by_key.get(&(entry.source, entry.number)) {
                Some(&lane) => out.resolved.push(ResolvedLane {
                    course_index: index,
                    lane,
                }),
                None => out.unresolved.push((index, *entry)),
            }
        }
        for pair in out.resolved.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if b.course_index == a.course_index + 1 && !self.successors(a.lane).contains(&b.lane) {
                out.broken_joins.push(b.course_index);
            }
        }
        out
    }
}
