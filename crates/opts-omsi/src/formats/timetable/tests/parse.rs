//! Parsing of the stop list, trip, line, track and station-link files.

use super::super::*;
use super::fixtures::*;

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

#[test]
fn station_links_name_the_stations_give_the_length_and_list_the_pieces_of_path() {
    let l = parse_stn_links(LINKS.as_bytes());
    assert_eq!(l.len(), 2);
    assert_eq!(
        (l[0].from_name.as_str(), l[0].to_name.as_str()),
        ("Ponttor", "Bushof")
    );
    assert_eq!((l[0].from_id, l[0].to_id, l[0].length_m), (100, 101, 600.0));
    assert_eq!(
        (l[0].from_a, l[0].to_a, l[0].from_b, l[0].to_b),
        (1.13, 4.06, 87.0, 64.8)
    );
    assert_eq!(l[0].last_entry, 1);
    assert_eq!(l[0].entries.len(), 2);
    assert_eq!(l[0].entries.iter().map(|e| e.length_m).sum::<f32>(), 600.0);
    assert_eq!(l[0].entries[0].id, 17754);
    assert_eq!(l[1].entries[0].f, 1);
    // The second link starts where the first ends.
    assert_eq!(l[1].from_id, l[0].to_id);
}
