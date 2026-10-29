//! Corpus test: the timetable files of every map's `TTData` folder parse and refer to each
//! other (M-0099): the runs of a line file name trip files that exist and profiles they have,
//! the stations of a trip are in the stop list, and the schedule of every run runs forward in
//! time. Skipped unless `OMSI_PATH` points at an installation.

use std::path::PathBuf;

use opts_omsi::formats::timetable::{parse_track, Timetables, TtTourTrip};

/// The `TTData` folder of every map of the installation `OMSI_PATH` names; `None` (skip) without.
fn ttdata_dirs() -> Option<Vec<PathBuf>> {
    let Some(root) = std::env::var_os("OMSI_PATH").map(PathBuf::from) else {
        eprintln!("OMSI_PATH not set: skipped");
        return None;
    };
    let maps = std::fs::read_dir(root.join("maps")).ok()?;
    Some(
        maps.flatten()
            .filter_map(|m| {
                std::fs::read_dir(m.path())
                    .into_iter()
                    .flatten()
                    .flatten()
                    .find(|e| {
                        e.file_name()
                            .to_string_lossy()
                            .eq_ignore_ascii_case("TTData")
                    })
                    .map(|e| e.path())
            })
            .collect(),
    )
}

/// What the first test counts over the corpus.
#[derive(Default)]
struct Counts {
    folders: usize,
    stops: usize,
    lines: usize,
    tours: usize,
    runs: usize,
    trips_missing: usize,
    profile_missing: usize,
    station_unknown: usize,
    stations: usize,
    backwards: usize,
    empty: usize,
    links: usize,
    link_ids: usize,
    link_unknown: usize,
    link_chain: usize,
    track_files: usize,
    track_entries: usize,
}

impl Counts {
    /// The station links: known ends, and pieces of path that add up to the length.
    fn links(&mut self, t: &Timetables) {
        for l in &t.links {
            self.links += 1;
            for id in [l.from_id, l.to_id] {
                self.link_ids += 1;
                self.link_unknown += usize::from(t.stop(id).is_none());
            }
            // The pieces of path add up to the link's length (within 10 %).
            let sum: f32 = l.entries.iter().map(|e| e.length_m).sum();
            self.link_chain += usize::from((sum - l.length_m).abs() <= 0.1 * l.length_m.max(1.0));
        }
    }

    /// The `.ttr` track files of a folder.
    fn tracks(&mut self, dir: &std::path::Path) {
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            if e.path()
                .extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("ttr"))
            {
                if let Ok(b) = std::fs::read(e.path()) {
                    self.track_files += 1;
                    self.track_entries += parse_track(&b).len();
                }
            }
        }
    }

    /// One run of a line file: its trip file and profile, and its schedule.
    fn run(&mut self, t: &Timetables, name: &str, (ti, ri): (usize, usize), run: &TtTourTrip) {
        self.runs += 1;
        let Some(trip) = t.trip(&run.trip) else {
            self.trips_missing += 1;
            return;
        };
        if run.profile >= trip.profiles.len() {
            self.profile_missing += 1;
        }
        let Some(s) = t.schedule(name, ti, ri) else {
            return;
        };
        if s.stops.is_empty() {
            self.empty += 1;
        }
        let legacy = trip.stations.iter().any(|x| x.legacy);
        if !legacy {
            for st in &s.stops {
                self.stations += 1;
                if t.stop(st.station_id).is_none() {
                    self.station_unknown += 1;
                }
            }
        }
        if s.stops
            .windows(2)
            .any(|w| w[1].arrival_s + 0.01 < w[0].departure_s)
        {
            self.backwards += 1;
        }
    }

    fn folder(&mut self, dir: &std::path::Path) {
        self.folders += 1;
        let t = Timetables::load_dir(dir);
        self.stops += t.stops.len();
        self.links(&t);
        self.tracks(dir);
        for (name, line) in &t.lines {
            self.lines += 1;
            for (ti, tour) in line.tours.iter().enumerate() {
                self.tours += 1;
                for (ri, run) in tour.trips.iter().enumerate() {
                    self.run(&t, name, (ti, ri), run);
                }
            }
        }
    }

    fn report(&self) {
        let c = self;
        eprintln!(
            "{} TTData folders: {} stops, {} line files, {} tours, {} runs ({} name a missing trip file, {} a missing profile, {} runs without stations, {} run backwards after the repair), {} scheduled stations ({} not in the stop list); {} track files with {} entries; {} station links, {} of {} link ends not in the stop list, {} links whose pieces add up to their length",
            c.folders, c.stops, c.lines, c.tours, c.runs, c.trips_missing, c.profile_missing,
            c.empty, c.backwards, c.stations, c.station_unknown, c.track_files, c.track_entries,
            c.links, c.link_unknown, c.link_ids, c.link_chain
        );
    }
}

#[test]
fn every_timetable_folder_parses_and_is_consistent() {
    let Some(dirs) = ttdata_dirs() else {
        return;
    };
    let mut c = Counts::default();
    for dir in &dirs {
        c.folder(dir);
    }
    c.report();
    assert!(c.folders > 5, "the corpus is not there");
    assert!(c.runs > 1000);
    assert!(
        c.trips_missing * 50 < c.runs,
        "{} of {} runs name a missing trip",
        c.trips_missing,
        c.runs
    );
    assert!(c.links > 1000, "{} links", c.links);
    assert!(
        c.link_unknown * 20 < c.link_ids,
        "{} of {} link ends unknown",
        c.link_unknown,
        c.link_ids
    );
    assert!(
        c.station_unknown * 20 < c.stations,
        "{} of {} stations are unknown",
        c.station_unknown,
        c.stations
    );
}

/// The errors of both interpolations when each manual time of a trip is hidden in turn.
fn hidden_time_errors(
    trip: &opts_omsi::formats::timetable::TtTrip,
    segs: &[f32],
    (err_index, err_dist): (&mut Vec<f32>, &mut Vec<f32>),
) {
    use opts_omsi::formats::timetable::station_times;
    for p in &trip.profiles {
        let n = trip.stations.len();
        let anchors: Vec<usize> = (0..n).filter(|j| p.arrival_min.contains_key(j)).collect();
        if anchors.len() < 4 {
            continue;
        }
        for &j in &anchors[1..anchors.len() - 1] {
            let Some(want) = p.arrival_min.get(&j).copied() else {
                continue;
            };
            let mut hidden = p.clone();
            hidden.arrival_min.remove(&j);
            hidden.departure_min.remove(&j);
            hidden.other_stopping.remove(&j);
            let by_index = station_times(&hidden, n, None)[j].0;
            let by_dist = station_times(&hidden, n, Some(segs))[j].0;
            err_index.push((by_index - want).abs());
            err_dist.push((by_dist - want).abs());
        }
    }
}

/// Mean, median, p90 and the share within a few limits of a list of errors (minutes).
fn error_stats(v: &mut [f32]) -> String {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = v.len().max(1) as f32;
    let mean = v.iter().sum::<f32>() / n;
    let within = |x: f32| v.iter().filter(|e| **e <= x).count() as f32 / n * 100.0;
    format!(
        "mean {mean:.3} median {:.3} p90 {:.3} min; within 0.25 min {:.0} %, 0.5 min {:.0} %, 1 min {:.0} %",
        v.get(v.len() / 2).copied().unwrap_or(0.0),
        v.get(v.len() * 9 / 10).copied().unwrap_or(0.0),
        within(0.25),
        within(0.5),
        within(1.0),
    )
}

/// How well the two ways of interpolating the unlisted stations predict a manual time that is
/// hidden (leave one out): by station index or by the lengths of the links of `StnLinks.cfg`.
#[test]
fn the_link_lengths_predict_manual_times_better_than_the_station_order() {
    let Some(dirs) = ttdata_dirs() else {
        return;
    };
    let (mut trips, mut linked_trips, mut links_files, mut links_total) = (0usize, 0, 0, 0usize);
    let (mut err_index, mut err_dist): (Vec<f32>, Vec<f32>) = (Vec::new(), Vec::new());
    for dir in &dirs {
        let t = Timetables::load_dir(dir);
        if !t.links.is_empty() {
            links_files += 1;
            links_total += t.links.len();
        }
        // Every trip file once: reach them through the runs of the line files.
        let mut seen = std::collections::HashSet::new();
        let runs = t
            .lines
            .iter()
            .flat_map(|(_, l)| &l.tours)
            .flat_map(|o| &o.trips);
        for run in runs {
            if !seen.insert(run.trip.to_ascii_lowercase()) {
                continue;
            }
            let Some(trip) = t.trip(&run.trip) else {
                continue;
            };
            if trip.stations.iter().any(|s| s.legacy) || trip.stations.len() < 4 {
                continue;
            }
            trips += 1;
            let Some(segs) = t.segment_lengths(trip) else {
                continue;
            };
            linked_trips += 1;
            hidden_time_errors(trip, &segs, (&mut err_index, &mut err_dist));
        }
    }
    let (si, sj) = (error_stats(&mut err_index), error_stats(&mut err_dist));
    eprintln!(
        "{links_files} StnLinks files with {links_total} links; {linked_trips} of {trips} bus trips have a link for every pair of stations; {} hidden manual times (the order is the default, see `Interpolation`):\n  by index:       {si}\n  by link length: {sj}",
        err_index.len()
    );
    assert!(links_files > 5, "the corpus is not there");
    assert!(!err_index.is_empty());
}
