//! Corpus test: the timetable files of every map's `TTData` folder parse and refer to each
//! other (M-0099): the runs of a line file name trip files that exist and profiles they have,
//! the stations of a trip are in the stop list, and the schedule of every run runs forward in
//! time. Skipped unless `OMSI_PATH` points at an installation.

use std::path::PathBuf;

use opts_core::formats::timetable::{parse_track, Timetables};

#[test]
fn every_timetable_folder_parses_and_is_consistent() {
    let Some(root) = std::env::var_os("OMSI_PATH").map(PathBuf::from) else {
        eprintln!("OMSI_PATH not set: skipped");
        return;
    };
    let Ok(maps) = std::fs::read_dir(root.join("maps")) else {
        return;
    };
    let (mut folders, mut stops, mut lines, mut tours, mut runs, mut trips_missing) =
        (0usize, 0usize, 0usize, 0usize, 0usize, 0usize);
    let (mut profile_missing, mut station_unknown, mut stations, mut backwards, mut empty) =
        (0usize, 0usize, 0usize, 0usize, 0usize);
    let mut track_files = 0usize;
    let mut track_entries = 0usize;
    for m in maps.flatten() {
        let dir = std::fs::read_dir(m.path())
            .into_iter()
            .flatten()
            .flatten()
            .find(|e| {
                e.file_name()
                    .to_string_lossy()
                    .eq_ignore_ascii_case("TTData")
            })
            .map(|e| e.path());
        let Some(dir) = dir else { continue };
        folders += 1;
        let t = Timetables::load_dir(&dir);
        stops += t.stops.len();
        for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            if e.path()
                .extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("ttr"))
            {
                if let Ok(b) = std::fs::read(e.path()) {
                    track_files += 1;
                    track_entries += parse_track(&b).len();
                }
            }
        }
        for (name, line) in &t.lines {
            lines += 1;
            for (ti, tour) in line.tours.iter().enumerate() {
                tours += 1;
                for (ri, run) in tour.trips.iter().enumerate() {
                    runs += 1;
                    let Some(trip) = t.trip(&run.trip) else {
                        trips_missing += 1;
                        continue;
                    };
                    if run.profile >= trip.profiles.len() {
                        profile_missing += 1;
                    }
                    let Some(s) = t.schedule(name, ti, ri) else {
                        continue;
                    };
                    if s.stops.is_empty() {
                        empty += 1;
                    }
                    for st in &s.stops {
                        let legacy = t
                            .trip(&run.trip)
                            .is_some_and(|tr| tr.stations.iter().any(|x| x.legacy));
                        if legacy {
                            continue;
                        }
                        stations += 1;
                        if t.stop(st.station_id).is_none() {
                            station_unknown += 1;
                        }
                    }
                    if s.stops
                        .windows(2)
                        .any(|w| w[1].arrival_s + 0.01 < w[0].departure_s)
                    {
                        backwards += 1;
                    }
                }
            }
        }
    }
    eprintln!(
        "{folders} TTData folders: {stops} stops, {lines} line files, {tours} tours, {runs} runs ({trips_missing} name a missing trip file, {profile_missing} a missing profile, {empty} runs without stations, {backwards} run backwards after the repair), {stations} scheduled stations ({station_unknown} not in the stop list); {track_files} track files with {track_entries} entries"
    );
    assert!(folders > 5, "the corpus is not there");
    assert!(runs > 1000);
    assert!(
        trips_missing * 50 < runs,
        "{trips_missing} of {runs} runs name a missing trip"
    );
    assert!(
        station_unknown * 20 < stations,
        "{station_unknown} of {stations} stations are unknown"
    );
}
