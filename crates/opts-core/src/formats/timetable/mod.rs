//! Timetable files of a map's `TTData` folder (M-0099): the stop list `Busstops.cfg`, the trip
//! files `.ttp`, the line files `.ttl` and the rail track files `.ttr`
//! (`docs/src/formats/text/timetable.md`).
//!
//! The files are keyword text like the other OMSI configuration files, but their values sit at
//! fixed positions after the keyword, and an empty line is a value (an unnamed stop, an empty
//! title): the parsers read positionally and never skip a blank line inside a record.

use std::collections::BTreeMap;

use crate::formats::text::decode_bytes;

/// The lines of a file without carriage returns.
fn lines_of(bytes: &[u8]) -> Vec<String> {
    decode_bytes(bytes)
        .lines()
        .map(|l| l.trim_end_matches('\r').to_string())
        .collect()
}

/// The keyword of a line (`[addtrip]` gives `addtrip`), lower case; text after the closing
/// bracket (`[Postdrohne] Hildegardplatz`) is ignored.
fn keyword(line: &str) -> Option<String> {
    let t = line.trim_start();
    let rest = t.strip_prefix('[')?;
    let end = rest.find(']')?;
    Some(rest[..end].trim().to_ascii_lowercase())
}

fn at(lines: &[String], i: usize) -> &str {
    lines.get(i).map_or("", |s| s.trim())
}

fn num<T: std::str::FromStr + Default>(s: &str) -> T {
    s.trim().replace(',', ".").parse().unwrap_or_default()
}

// ---------------------------------------------------------------------------------------------
// Busstops.cfg

/// A stop of the map's timetable stop list.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TtBusStop {
    /// Stop name, as the hof's stop idents spell it (empty for an unnamed platform).
    pub name: String,
    /// Second value of the record (350 distinct values over 5,506 stops of 46 maps: a stop
    /// group or area number ❓).
    pub group: i32,
    /// Station id, the number a trip file's `[station_typ2]` names.
    pub id: u32,
    /// Standing time at the stop (s ❓: 0, 1, 2, 3, 5 and 10 are the common values).
    pub dwell_s: f32,
    /// Fifth value (0 for 5,505 of 5,506 stops ❓).
    pub flag: i32,
    /// Sixth value: a `Chrono.cfg` path in one record, empty otherwise ❓.
    pub chrono: String,
}

/// Parses `Busstops.cfg`.
#[must_use]
pub fn parse_busstops(bytes: &[u8]) -> Vec<TtBusStop> {
    let l = lines_of(bytes);
    let mut out = Vec::new();
    let mut i = 0;
    while i < l.len() {
        if keyword(&l[i]).as_deref() == Some("busstop") {
            out.push(TtBusStop {
                name: at(&l, i + 1).to_string(),
                group: num(at(&l, i + 2)),
                id: num(at(&l, i + 3)),
                dwell_s: num(at(&l, i + 4)),
                flag: num(at(&l, i + 5)),
                chrono: at(&l, i + 6).to_string(),
            });
            i += 7;
        } else {
            i += 1;
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------
// .ttp

/// One station of a trip.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TtStation {
    /// Station id (a `Busstops.cfg` stop for `[station_typ2]`).
    pub id: u32,
    /// Name of a legacy `[station]` record (rail), empty for `[station_typ2]`.
    pub name: String,
    /// A legacy `[station]` record (rail and tram timetables): its id is not in `Busstops.cfg`.
    pub legacy: bool,
}

/// A timing profile of a trip: manual arrival and departure offsets per station.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TtProfile {
    /// Profile name (`standard`, `Blockverkehr`, or a number).
    pub name: String,
    /// Second value of the record: the running time of the trip in minutes (equals the last
    /// manual time in the files checked).
    pub total_min: f32,
    /// Arrival offsets after the trip's start, minutes, by station index.
    pub arrival_min: BTreeMap<usize, f32>,
    /// Departure offsets after the trip's start, minutes, by station index.
    pub departure_min: BTreeMap<usize, f32>,
    /// `[profile_otherstopping]`: a number per station index (the standing time in seconds ❓).
    pub other_stopping: BTreeMap<usize, f32>,
}

/// A trip file: a run over a list of stations with one or more timing profiles.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TtTrip {
    /// First value after `[trip]` (the train name of a rail trip, empty for bus trips).
    pub name: String,
    /// Destination text (second value of a bus trip).
    pub title: String,
    /// Line text (third value of a bus trip).
    pub line: String,
    /// The stations in running order.
    pub stations: Vec<TtStation>,
    /// The timing profiles.
    pub profiles: Vec<TtProfile>,
}

/// Parses a `.ttp` trip file.
#[must_use]
pub fn parse_trip(bytes: &[u8]) -> TtTrip {
    let l = lines_of(bytes);
    let mut trip = TtTrip::default();
    let mut i = 0;
    while i < l.len() {
        let Some(k) = keyword(&l[i]) else {
            i += 1;
            continue;
        };
        match k.as_str() {
            "trip" => {
                trip.name = at(&l, i + 1).to_string();
                trip.title = at(&l, i + 2).to_string();
                trip.line = at(&l, i + 3).to_string();
                i += 4;
            }
            "station_typ2" => {
                trip.stations.push(TtStation {
                    id: num(at(&l, i + 1)),
                    name: String::new(),
                    legacy: false,
                });
                i += 2;
            }
            "station" => {
                trip.stations.push(TtStation {
                    id: num(at(&l, i + 1)),
                    name: at(&l, i + 3).to_string(),
                    legacy: true,
                });
                i += 9;
            }
            "profile" => {
                trip.profiles.push(TtProfile {
                    name: at(&l, i + 1).to_string(),
                    total_min: num(at(&l, i + 2)),
                    ..TtProfile::default()
                });
                i += 3;
            }
            "profile_man_arr_time" | "profile_man_dep_time" | "profile_otherstopping" => {
                let (idx, value): (usize, f32) = (num(at(&l, i + 1)), num(at(&l, i + 2)));
                if let Some(p) = trip.profiles.last_mut() {
                    match k.as_str() {
                        "profile_man_arr_time" => p.arrival_min.insert(idx, value),
                        "profile_man_dep_time" => p.departure_min.insert(idx, value),
                        _ => p.other_stopping.insert(idx, value),
                    };
                }
                i += 3;
            }
            _ => i += 1,
        }
    }
    trip
}

// ---------------------------------------------------------------------------------------------
// .ttl

/// One run of a tour: a trip, its timing profile and the minute of the day it starts.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TtTourTrip {
    /// Trip file name without extension.
    pub trip: String,
    /// Index of the timing profile in the trip file (0-based).
    pub profile: usize,
    /// Start of the trip (minutes after midnight; the `Dep.:` comment line says the same).
    pub start_min: f32,
}

/// A tour: a named sequence of trips for one vehicle.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TtTour {
    /// Tour name (`MOFR_NORD`).
    pub name: String,
    /// Second value: the vehicle group the tour is for (`Gelenk ASEAG`).
    pub vehicle_group: String,
    /// Third value (799 for the Aachen tours ❓).
    pub number: i32,
    /// The runs in the order of the file.
    pub trips: Vec<TtTourTrip>,
}

/// A line file: tours of one line (or group of lines).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TtLine {
    /// `[priority]`.
    pub priority: i32,
    /// `[userallowed]` present: the player may drive the tours of this file.
    pub user_allowed: bool,
    /// The tours.
    pub tours: Vec<TtTour>,
}

/// Parses a `.ttl` line file.
#[must_use]
pub fn parse_line(bytes: &[u8]) -> TtLine {
    let l = lines_of(bytes);
    let mut line = TtLine::default();
    let mut i = 0;
    while i < l.len() {
        let Some(k) = keyword(&l[i]) else {
            i += 1;
            continue;
        };
        match k.as_str() {
            "userallowed" => {
                line.user_allowed = true;
                i += 1;
            }
            "priority" => {
                line.priority = num(at(&l, i + 1));
                i += 2;
            }
            "newtour" => {
                line.tours.push(TtTour {
                    name: at(&l, i + 1).to_string(),
                    vehicle_group: at(&l, i + 2).to_string(),
                    number: num(at(&l, i + 3)),
                    trips: Vec::new(),
                });
                i += 4;
            }
            "addtrip" => {
                if let Some(t) = line.tours.last_mut() {
                    t.trips.push(TtTourTrip {
                        trip: at(&l, i + 1).to_string(),
                        profile: num(at(&l, i + 2)),
                        start_min: num(at(&l, i + 3)),
                    });
                }
                i += 4;
            }
            _ => i += 1,
        }
    }
    line
}

// ---------------------------------------------------------------------------------------------
// .ttr

/// One entry of a rail track file (`[track_entry]`: six values after the keyword).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TtTrackEntry {
    /// First value (a track or path id ❓).
    pub id: u32,
    /// Second to sixth values (0, 92, 41, a length in metres and 0 in the sample); their
    /// meaning is not known ❓.
    pub values: [f32; 5],
}

/// Parses a `.ttr` track file (bus maps ignore them; rail is M-0098).
#[must_use]
pub fn parse_track(bytes: &[u8]) -> Vec<TtTrackEntry> {
    let l = lines_of(bytes);
    let mut out = Vec::new();
    let mut i = 0;
    while i < l.len() {
        if keyword(&l[i]).as_deref() == Some("track_entry") {
            out.push(TtTrackEntry {
                id: num(at(&l, i + 1)),
                values: [
                    num(at(&l, i + 2)),
                    num(at(&l, i + 3)),
                    num(at(&l, i + 4)),
                    num(at(&l, i + 5)),
                    num(at(&l, i + 6)),
                ],
            });
            i += 7;
        } else {
            i += 1;
        }
    }
    out
}

mod model;
pub use model::{ActiveSchedule, ScheduledStop, Timetables};

#[cfg(test)]
mod tests;
