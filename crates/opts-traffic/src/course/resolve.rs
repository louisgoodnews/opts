//! The course of a trip as paths of the map: built from the timetable files, resolved against the
//! lane graph by [`Lane::key`](crate::graph::Lane).
//!
//! Field meanings are the open question Q-T04 (`docs/src/formats/map/timetables.md`): the first
//! value of an entry is the element's ID code, the second the direction (taken here as `1` =
//! against the path, anything else = along it, ❓), the third the path number (`number`). A
//! [`CourseLane`] keeps the three values; nothing is guessed beyond that mapping, and the corpus
//! check (milestone M-0105, step 1) decides whether it holds.

use opts_core::formats::timetable::{StnLinkEntry, Timetables, TtTrackEntry, TtTrip};

use opts_traffic_lanes::graph::CourseLane;

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
