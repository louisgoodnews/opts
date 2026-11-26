//! # OPTS OMSI timetable readers (`opts-omsi-timetable`)
//!
//! Timetable files of a map's `TTData` folder (M-0099): the stop list `Busstops.cfg`, the trip
//! files `.ttp`, the line files `.ttl` and the rail track files `.ttr`
//! (`docs/src/formats/text/timetable.md`).
//!
//! The files are keyword text like the other OMSI configuration files, but their values sit at
//! fixed positions after the keyword, and an empty line is a value (an unnamed stop, an empty
//! title): the parsers read positionally and never skip a blank line inside a record.
//!
//! Split out of `opts-omsi-text` (ISS-087, stage 1). `opts-omsi-text` re-exports this crate as
//! `formats::timetable`, so the old paths keep working. Pure in-memory parsers: no file I/O, no
//! engine, no OMSI binaries (ADR-0002).

#![warn(missing_docs)]
#![deny(unsafe_code)]

mod busstops;
mod line;
mod stn_links;
mod text;
mod track;
mod trip;

pub use busstops::{parse_busstops, TtBusStop};
pub use line::{parse_line, TtLine, TtTour, TtTourTrip};
pub use stn_links::{parse_stn_links, StnLink, StnLinkEntry};
pub use track::{parse_track, TtTrackEntry};
pub use trip::{parse_trip, TtProfile, TtStation, TtTrip};

mod interpolation;
mod load;
mod model;
mod query;

pub use interpolation::{station_times, Interpolation};
pub use model::{ActiveSchedule, ScheduledStop, Timetables};

#[cfg(test)]
mod tests;
