use super::*;

const TTP: &str = "-----------------------\r\nTime Table Trip File\r\n-----------------------\r\n\r\n[trip]\r\n\r\nHoengen I G A\r\n11\r\n\r\n.........................\r\n        Stations\r\n.........................\r\n\r\n[station_typ2]\r\n100\r\n\r\n[station_typ2]\r\n101\r\n\r\n[station_typ2]\r\n102\r\n\r\n[station_typ2]\r\n103\r\n\r\n[station_typ2]\r\n104\r\n\r\n[profile]\r\nstandard\r\n8.000\r\n\r\n[profile_man_arr_time]\r\n0\r\n0.000\r\n[profile_man_arr_time]\r\n2\r\n2.000\r\n[profile_otherstopping]\r\n2\r\n30\r\n[profile_man_dep_time]\r\n2\r\n3.000\r\n[profile_man_arr_time]\r\n4\r\n8.000\r\n[profile]\r\nBlockverkehr\r\n10.000\r\n";

const TTL: &str = "------------------------\r\nTime Table Line File\r\n------------------------\r\n\r\n[userallowed]\r\n\r\n[priority]\r\n2\r\n------------------------------------\r\n\r\n[newtour]\r\nMOFR_NORD\r\nGelenk ASEAG\r\n799\r\n\r\n------------------------------------\r\n\r\n  Dep.: 4:54:0\r\n[addtrip]\r\n11_IGA\r\n1\r\n294.000\r\n\r\n  Dep.: 5:24:0\r\n[addtrip]\r\n11_IGA\r\n0\r\n324.000\r\n";

const STOPS: &str = "---------------------------\r\nTime Table BusStopList File\r\n---------------------------\r\n\r\n[busstop]\r\nHeuvel\r\n79\r\n100\r\n5.0000000000\r\n0\r\n\r\n\r\n[busstop]\r\n\r\n58\r\n101\r\n2.0000000000\r\n0\r\n\r\n";

#[test]
fn the_stop_list_keeps_blank_names_and_the_station_ids() {
    let s = parse_busstops(STOPS.as_bytes());
    assert_eq!(s.len(), 2);
    assert_eq!(s[0].name, "Heuvel");
    assert_eq!((s[0].group, s[0].id, s[0].dwell_s), (79, 100, 5.0));
    assert_eq!(s[1].name, "", "an unnamed platform");
    assert_eq!((s[1].group, s[1].id, s[1].dwell_s), (58, 101, 2.0));
}

#[test]
fn a_trip_has_title_line_stations_and_profiles_with_manual_times() {
    let t = parse_trip(TTP.as_bytes());
    assert_eq!((t.title.as_str(), t.line.as_str()), ("Hoengen I G A", "11"));
    assert_eq!(t.name, "");
    assert_eq!(
        t.stations.iter().map(|s| s.id).collect::<Vec<_>>(),
        [100, 101, 102, 103, 104]
    );
    assert_eq!(t.profiles.len(), 2);
    assert_eq!(t.profiles[0].name, "standard");
    assert_eq!(t.profiles[0].total_min, 8.0);
    assert_eq!(t.profiles[0].arrival_min[&2], 2.0);
    assert_eq!(t.profiles[0].departure_min[&2], 3.0);
    assert_eq!(t.profiles[0].other_stopping[&2], 30.0);
    assert!(t.profiles[1].arrival_min.is_empty());
}

#[test]
fn a_line_file_has_tours_with_runs_and_the_player_flag() {
    let l = parse_line(TTL.as_bytes());
    assert!(l.user_allowed);
    assert_eq!(l.priority, 2);
    assert_eq!(l.tours.len(), 1);
    let t = &l.tours[0];
    assert_eq!(
        (t.name.as_str(), t.vehicle_group.as_str(), t.number),
        ("MOFR_NORD", "Gelenk ASEAG", 799)
    );
    assert_eq!(t.trips.len(), 2);
    assert_eq!(
        (
            t.trips[0].trip.as_str(),
            t.trips[0].profile,
            t.trips[0].start_min
        ),
        ("11_IGA", 1, 294.0)
    );
    assert_eq!(t.trips[1].start_min, 324.0);
}

#[test]
fn track_files_read_six_values_per_entry() {
    let ttr = "Time Table Track File\r\n\r\n   0:\r\n[track_entry]\r\n27342\r\n0\r\n92\r\n41\r\n300.0000000000\r\n0\r\n\r\n   1:\r\n[track_entry]\r\n27343\r\n0\r\n92\r\n42\r\n169.0\r\n0\r\n";
    let t = parse_track(ttr.as_bytes());
    assert_eq!(t.len(), 2);
    assert_eq!(t[0].id, 27342);
    assert_eq!(t[1].values[3], 169.0);
}

fn timetables() -> Timetables {
    Timetables::from_parts(
        parse_busstops(STOPS.as_bytes()),
        vec![("11".into(), parse_line(TTL.as_bytes()))],
        vec![("11_IGA".into(), parse_trip(TTP.as_bytes()))],
    )
}

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
