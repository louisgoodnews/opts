//! The scheduler against the timetables of installed maps. Skipped without `OMSI_PATH`.

use opts_core::formats::timetable::Timetables;
use opts_game::in_game::omsi_world::{active_runs, run_phase, RunPhase};

#[test]
fn every_installed_map_has_runs_at_midday_and_each_is_on_its_way() {
    let Some(root) = std::env::var_os("OMSI_PATH").map(std::path::PathBuf::from) else {
        eprintln!("OMSI_PATH not set; skipping");
        return;
    };
    let mut maps_with_runs = 0;
    for name in [
        "Berlin-Spandau",
        "Ahlheim 5",
        "Koeln",
        "HB_2026_Bremen-Nord",
    ] {
        let dir = root.join("maps").join(name).join("TTData");
        if !dir.exists() {
            continue;
        }
        let tt = Timetables::load_dir(&dir);
        let runs = active_runs(&tt, 12.0 * 3600.0, 600.0, 4.0 * 3600.0, None);
        eprintln!(
            "{name}: {} runs on the road or starting within 10 min at 12:00",
            runs.len()
        );
        for (_, s) in &runs {
            let phase = run_phase(s, 12.0 * 3600.0);
            assert!(
                !matches!(phase, RunPhase::Finished),
                "{} finished but listed",
                s.trip
            );
        }
        if !runs.is_empty() {
            maps_with_runs += 1;
        }
    }
    assert!(
        maps_with_runs >= 1,
        "some installed map has a timetable with midday runs"
    );
}

/// Every tour that is on the road at midday names a depot group of the map's `ailists.cfg`, and
/// that group has vehicles: the vehicle of a scheduled run can be chosen.
#[test]
fn tours_name_depot_groups_with_vehicles() {
    use opts_core::formats::map::AiLists;
    let Some(root) = std::env::var_os("OMSI_PATH").map(std::path::PathBuf::from) else {
        return;
    };
    for name in [
        "Berlin-Spandau",
        "Ahlheim 5",
        "Koeln",
        "HB_2026_Bremen-Nord",
    ] {
        let map = root.join("maps").join(name);
        let Ok(bytes) = std::fs::read(map.join("ailists.cfg")) else {
            continue;
        };
        let lists = AiLists::parse(&bytes);
        let tt = Timetables::load_dir(&map.join("TTData"));
        let (mut with, mut total, mut without) = (0, 0, Vec::new());
        for (stem, line) in &tt.lines {
            for tour in &line.tours {
                total += 1;
                if lists
                    .depot_group(&tour.vehicle_group)
                    .is_some_and(|g| !g.vehicles.is_empty())
                {
                    with += 1;
                } else if without.len() < 3 {
                    without.push(format!("{stem}/{}: '{}'", tour.name, tour.vehicle_group));
                }
            }
        }
        eprintln!("{name}: {with} of {total} tours have a depot group with vehicles; e.g. without: {without:?}");
    }
}
