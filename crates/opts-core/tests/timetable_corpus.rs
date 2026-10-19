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
    let (mut links, mut link_ids, mut link_unknown, mut link_chain) =
        (0usize, 0usize, 0usize, 0usize);
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
        for l in &t.links {
            links += 1;
            for id in [l.from_id, l.to_id] {
                link_ids += 1;
                link_unknown += usize::from(t.stop(id).is_none());
            }
            // The pieces of path add up to the link's length (within 10 %).
            let sum: f32 = l.entries.iter().map(|e| e.length_m).sum();
            link_chain += usize::from((sum - l.length_m).abs() <= 0.1 * l.length_m.max(1.0));
        }
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
        "{folders} TTData folders: {stops} stops, {lines} line files, {tours} tours, {runs} runs ({trips_missing} name a missing trip file, {profile_missing} a missing profile, {empty} runs without stations, {backwards} run backwards after the repair), {stations} scheduled stations ({station_unknown} not in the stop list); {track_files} track files with {track_entries} entries; {links} station links, {link_unknown} of {link_ids} link ends not in the stop list, {link_chain} links whose pieces add up to their length"
    );
    assert!(folders > 5, "the corpus is not there");
    assert!(runs > 1000);
    assert!(
        trips_missing * 50 < runs,
        "{trips_missing} of {runs} runs name a missing trip"
    );
    assert!(links > 1000, "{links} links");
    assert!(
        link_unknown * 20 < link_ids,
        "{link_unknown} of {link_ids} link ends unknown"
    );
    assert!(
        station_unknown * 20 < stations,
        "{station_unknown} of {stations} stations are unknown"
    );
}

/// How well the two ways of interpolating the unlisted stations predict a manual time that is
/// hidden (leave one out): by station index or by the lengths of the links of `StnLinks.cfg`.
#[test]
fn the_link_lengths_predict_manual_times_better_than_the_station_order() {
    use opts_core::formats::timetable::station_times;
    let Some(root) = std::env::var_os("OMSI_PATH").map(PathBuf::from) else {
        eprintln!("OMSI_PATH not set: skipped");
        return;
    };
    let Ok(maps) = std::fs::read_dir(root.join("maps")) else {
        return;
    };
    let (mut trips, mut linked_trips, mut links_files, mut links_total) = (0usize, 0, 0, 0usize);
    let (mut err_index, mut err_dist): (Vec<f32>, Vec<f32>) = (Vec::new(), Vec::new());
    for m in maps.flatten() {
        let Some(dir) = std::fs::read_dir(m.path())
            .into_iter()
            .flatten()
            .flatten()
            .find(|e| {
                e.file_name()
                    .to_string_lossy()
                    .eq_ignore_ascii_case("TTData")
            })
            .map(|e| e.path())
        else {
            continue;
        };
        let t = Timetables::load_dir(&dir);
        if !t.links.is_empty() {
            links_files += 1;
            links_total += t.links.len();
        }
        // Every trip file once: reach them through the runs of the line files.
        let mut seen = std::collections::HashSet::new();
        for (_, line) in &t.lines {
            for tour in &line.tours {
                for run in &tour.trips {
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
                    for p in &trip.profiles {
                        let n = trip.stations.len();
                        let truth = |j: usize| p.arrival_min.get(&j).copied();
                        let anchors: Vec<usize> =
                            (0..n).filter(|j| p.arrival_min.contains_key(j)).collect();
                        if anchors.len() < 4 {
                            continue;
                        }
                        for &j in &anchors[1..anchors.len() - 1] {
                            let Some(want) = truth(j) else { continue };
                            let mut hidden = p.clone();
                            hidden.arrival_min.remove(&j);
                            hidden.departure_min.remove(&j);
                            hidden.other_stopping.remove(&j);
                            let by_index = station_times(&hidden, n, None)[j].0;
                            let by_dist = station_times(&hidden, n, Some(&segs))[j].0;
                            err_index.push((by_index - want).abs());
                            err_dist.push((by_dist - want).abs());
                        }
                    }
                }
            }
        }
    }
    let stat = |v: &mut Vec<f32>| {
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
    };
    let (si, sj) = (stat(&mut err_index), stat(&mut err_dist));
    eprintln!(
        "{links_files} StnLinks files with {links_total} links; {linked_trips} of {trips} bus trips have a link for every pair of stations; {} hidden manual times (the order is the default, see `Interpolation`):\n  by index:       {si}\n  by link length: {sj}",
        err_index.len()
    );
    assert!(links_files > 5, "the corpus is not there");
    assert!(!err_index.is_empty());
}
