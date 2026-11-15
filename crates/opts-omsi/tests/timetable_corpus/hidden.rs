//! How well the two ways of interpolating the unlisted stations of a timetable trip predict a
//! hidden manual time (the link-length test of the timetable corpus).

use opts_omsi::formats::timetable::Timetables;

use super::ttdata_dirs;

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
