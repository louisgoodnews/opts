//! Schedules of a timetable: run times, manual times, interpolation and link distances.

use super::super::*;
use super::fixtures::*;

#[test]
fn a_schedule_takes_the_runs_start_and_the_profiles_times() {
    // Run 1 uses profile 1 ("Blockverkehr", no manual times): first at 0, last at 10 minutes,
    // the stations between interpolated by index.
    let s = timetables().schedule("11", 0, 0).expect("schedule");
    assert_eq!(
        (s.tour.as_str(), s.title.as_str(), s.line.as_str()),
        ("MOFR_NORD", "Hoengen I G A", "11")
    );
    assert_eq!(s.start_s, 294.0 * 60.0);
    assert_eq!(s.stops.len(), 5);
    let mins: Vec<f32> = s
        .stops
        .iter()
        .map(|x| (x.arrival_s - s.start_s) / 60.0)
        .collect();
    assert_eq!(mins, [0.0, 2.5, 5.0, 7.5, 10.0]);
    assert_eq!(s.stops[0].name, "Heuvel", "named from the stop list");
    assert_eq!(s.stops[1].name, "");
}

#[test]
fn manual_times_win_and_unlisted_stations_are_interpolated() {
    // Run 1 of the tour is the second `[addtrip]`, profile 0 ("standard").
    let s = timetables().schedule("11", 0, 1).expect("schedule");
    let arr: Vec<f32> = s
        .stops
        .iter()
        .map(|x| (x.arrival_s - s.start_s) / 60.0)
        .collect();
    let dep: Vec<f32> = s
        .stops
        .iter()
        .map(|x| (x.departure_s - s.start_s) / 60.0)
        .collect();
    // Stations 0, 2 and 4 have arrivals 0, 2 and 8; station 1 lies between 0 and 2, station 3
    // between 2 and 8; station 2 departs at its manual 3 minutes.
    assert_eq!(arr, [0.0, 1.0, 2.0, 5.0, 8.0]);
    assert_eq!(dep[2], 3.0);
    assert_eq!(dep[0], 0.0, "no dwell without `otherstopping`");
    assert_eq!(dep[4], 8.0, "the last station departs when it arrives");
    assert!(timetables().schedule("11", 0, 9).is_none());
    assert!(timetables().schedule("nope", 0, 0).is_none());
}

fn with_links() -> Timetables {
    timetables().with_links(parse_stn_links(LINKS.as_bytes()))
}

#[test]
fn a_link_is_found_in_either_direction_and_gives_the_distance_along_a_run() {
    let t = with_links();
    assert_eq!(t.link(100, 101).map(|l| l.length_m), Some(600.0));
    assert_eq!(
        t.link(101, 100).map(|l| l.length_m),
        Some(600.0),
        "the road is the same road"
    );
    assert!(t.link(100, 102).is_none());
    // The trip of the fixtures has five stations 100..104 in `TTP`, only the first pair linked:
    // no distance for the run.
    let s = t.schedule("11", 0, 1).expect("schedule");
    assert!(s.stops.iter().all(|x| x.distance_m.is_none()));
}

#[test]
fn the_interpolation_can_follow_the_link_lengths() {
    // Three stations 100, 101, 102 with arrivals listed for the first and the last only; the
    // links are 600 m and 1,400 m, so by distance the middle one is at 30 % of the time, by
    // order at 50 %.
    let ttp = "[trip]\n\nVaals\n33\n[station_typ2]\n100\n[station_typ2]\n101\n[station_typ2]\n102\n[profile]\nstandard\n10.0\n[profile_man_arr_time]\n0\n0.0\n[profile_man_arr_time]\n2\n10.0\n";
    let ttl = "[newtour]\nT\nBus\n1\n[addtrip]\nthree\n0\n480.000\n";
    let t = Timetables::from_parts(
        parse_busstops(STOPS.as_bytes()),
        vec![("3".into(), parse_line(ttl.as_bytes()))],
        vec![("three".into(), parse_trip(ttp.as_bytes()))],
    )
    .with_links(parse_stn_links(LINKS.as_bytes()));
    let mid = |mode| {
        let s = t.schedule_with("3", 0, 0, mode).expect("schedule");
        (s.stops[1].arrival_s - s.start_s) / 60.0
    };
    assert_eq!(mid(Interpolation::ByIndex), 5.0);
    assert!((mid(Interpolation::ByDistance) - 3.0).abs() < 1e-4);
    assert_eq!(
        mid(Interpolation::default()),
        5.0,
        "the order is the default"
    );
    let s = t.schedule("3", 0, 0).expect("schedule");
    assert_eq!(
        s.stops.iter().map(|x| x.distance_m).collect::<Vec<_>>(),
        [Some(0.0), Some(600.0), Some(2000.0)],
        "the distance along the run comes from the links"
    );
}
