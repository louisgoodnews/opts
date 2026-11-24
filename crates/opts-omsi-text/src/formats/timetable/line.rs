//! `.ttl` line files: the tours of a line and their runs.

use super::text::{at, keyword, lines_of, num};

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
