//! The timetables of a map and the schedule of one run derived from them.

use std::collections::HashMap;
use std::path::Path;

use super::{parse_busstops, parse_line, parse_trip, TtBusStop, TtLine, TtProfile, TtTour, TtTrip};

/// Everything a map's `TTData` folder holds that buses use: the stop list, the line files with
/// their tours and the trip files.
#[derive(Debug, Clone, Default)]
pub struct Timetables {
    /// `Busstops.cfg`.
    pub stops: Vec<TtBusStop>,
    /// `(file stem, line file)` in name order.
    pub lines: Vec<(String, TtLine)>,
    trips: HashMap<String, TtTrip>,
    stop_by_id: HashMap<u32, usize>,
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

fn find_ci(dir: &Path, name: &str) -> Option<std::path::PathBuf> {
    let want = name.to_ascii_lowercase();
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().to_ascii_lowercase() == want)
        })
}

impl Timetables {
    /// Reads `Busstops.cfg`, every `.ttl` and every `.ttp` of a `TTData` folder; a missing folder
    /// gives empty timetables, an unreadable file is skipped.
    #[must_use]
    pub fn load_dir(dir: &Path) -> Self {
        let mut t = Self::default();
        if let Some(p) = find_ci(dir, "Busstops.cfg") {
            if let Ok(b) = std::fs::read(p) {
                t.stops = parse_busstops(&b);
            }
        }
        let mut files: Vec<_> = std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .collect();
        files.sort();
        for p in files {
            let ext = p
                .extension()
                .map(|e| e.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default();
            let stem = p
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            let Ok(bytes) = std::fs::read(&p) else {
                continue;
            };
            match ext.as_str() {
                "ttl" => t.lines.push((stem, parse_line(&bytes))),
                "ttp" => {
                    t.trips
                        .insert(stem.to_ascii_lowercase(), parse_trip(&bytes));
                }
                _ => {}
            }
        }
        t.index();
        t
    }

    /// Timetables from parts (tests, tools).
    #[must_use]
    pub fn from_parts(
        stops: Vec<TtBusStop>,
        lines: Vec<(String, TtLine)>,
        trips: Vec<(String, TtTrip)>,
    ) -> Self {
        let mut t = Self {
            stops,
            lines,
            trips: trips
                .into_iter()
                .map(|(n, tr)| (n.to_ascii_lowercase(), tr))
                .collect(),
            stop_by_id: HashMap::new(),
        };
        t.index();
        t
    }

    fn index(&mut self) {
        self.stop_by_id.clear();
        for (i, s) in self.stops.iter().enumerate() {
            self.stop_by_id.entry(s.id).or_insert(i);
        }
    }

    /// Whether the folder held nothing usable.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// The stop with station id `id`.
    #[must_use]
    pub fn stop(&self, id: u32) -> Option<&TtBusStop> {
        self.stop_by_id.get(&id).and_then(|&i| self.stops.get(i))
    }

    /// A trip file by name (case-insensitive).
    #[must_use]
    pub fn trip(&self, name: &str) -> Option<&TtTrip> {
        self.trips.get(&name.to_ascii_lowercase())
    }

    /// The tour `tour` of line file `line_file` (case-insensitive stem).
    #[must_use]
    pub fn tour(&self, line_file: &str, tour: usize) -> Option<&TtTour> {
        self.lines
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(line_file))
            .and_then(|(_, l)| l.tours.get(tour))
    }

    /// The schedule of run `run` of tour `tour` in `line_file`: the trip's stations with the
    /// times its timing profile gives, counted from the run's start.
    #[must_use]
    pub fn schedule(&self, line_file: &str, tour: usize, run: usize) -> Option<ActiveSchedule> {
        let (stem, _) = self
            .lines
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(line_file))?;
        let t = self.tour(line_file, tour)?;
        let r = t.trips.get(run)?;
        let trip = self.trip(&r.trip)?;
        let start_s = r.start_min * 60.0;
        let profile = trip
            .profiles
            .get(r.profile)
            .or_else(|| trip.profiles.first());
        let times = profile.map_or_else(
            || vec![(0.0, 0.0); trip.stations.len()],
            |p| station_times(p, trip.stations.len()),
        );
        let stops = trip
            .stations
            .iter()
            .zip(times)
            .map(|(st, (a, d))| ScheduledStop {
                station_id: st.id,
                name: self
                    .stop(st.id)
                    .map_or_else(|| st.name.clone(), |s| s.name.clone()),
                arrival_s: start_s + a * 60.0,
                departure_s: start_s + d * 60.0,
            })
            .collect();
        Some(ActiveSchedule {
            line_file: stem.clone(),
            tour: t.name.clone(),
            trip: r.trip.clone(),
            title: trip.title.clone(),
            line: trip.line.clone(),
            start_s,
            stops,
        })
    }
}

/// Arrival and departure offsets (minutes after the start) of every station of a trip under a
/// profile.
///
/// The profile lists manual times for some stations only; the others are interpolated linearly
/// by station index between the neighbours that have one, the first station being at 0 and the
/// last at the profile's running time unless they have times of their own ❓ (the files give no
/// distances). Time never runs backwards: an arrival before the previous departure is moved to it. A station with only a departure arrives `otherstopping` seconds before it, one
/// with only an arrival departs `otherstopping` seconds after it; the last station departs when
/// it arrives.
fn station_times(p: &TtProfile, n: usize) -> Vec<(f32, f32)> {
    if n == 0 {
        return Vec::new();
    }
    let stop_min = |i: usize| p.other_stopping.get(&i).copied().unwrap_or(0.0) / 60.0;
    let mut reference: Vec<Option<f32>> = (0..n)
        .map(|i| {
            p.arrival_min
                .get(&i)
                .copied()
                .or_else(|| p.departure_min.get(&i).map(|d| d - stop_min(i)))
        })
        .collect();
    if reference[0].is_none() {
        reference[0] = Some(0.0);
    }
    if n > 1 && reference[n - 1].is_none() {
        reference[n - 1] = Some(
            p.total_min
                .max(reference.iter().flatten().copied().fold(0.0, f32::max)),
        );
    }
    let mut filled = vec![0.0f32; n];
    let mut prev = 0usize;
    for i in 0..n {
        if let Some(v) = reference[i] {
            filled[i] = v;
            // Interpolate the run of stations between the previous anchor and this one.
            for k in prev + 1..i {
                let f = (k - prev) as f32 / (i - prev) as f32;
                filled[k] = filled[prev] + (v - filled[prev]) * f;
            }
            prev = i;
        }
    }
    // Authored contradictions (an arrival of 1 minute at the station after one that departs at
    // 18.5, found in 5,751 of 291,253 runs) are repaired by never letting time run backwards.
    let mut out = Vec::with_capacity(n);
    let mut clock = 0.0f32;
    for (i, &reference) in filled.iter().enumerate() {
        let arrival = reference.max(clock);
        let departure = if i + 1 == n {
            arrival
        } else {
            p.departure_min
                .get(&i)
                .copied()
                .unwrap_or(arrival + stop_min(i))
                .max(arrival)
        };
        clock = departure;
        out.push((arrival, departure));
    }
    out
}
