//! Counts of the timetable corpus test: stops, runs, schedules, track files and station links, and
//! the consistency assertions over them.

use opts_omsi::formats::timetable::{parse_track, Timetables, TtTourTrip};

use super::ttdata_dirs;

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
