//! `.ttp` trip files: the stations of a trip and its timing profiles.

use std::collections::BTreeMap;

use super::text::{at, keyword, lines_of, num};

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
