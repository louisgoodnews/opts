//! The table of a map's timetables: which trips serve which stops, and what is left out.

use opts_core::formats::timetable::{
    Timetables, TtBusStop, TtLine, TtStation, TtTour, TtTourTrip, TtTrip,
};
use opts_sim::passengers::Destination;

use super::{destination_table, is_depot_run};

fn stop(id: u32, name: &str) -> TtBusStop {
    TtBusStop {
        name: name.to_string(),
        id,
        ..TtBusStop::default()
    }
}

fn trip(title: &str, line: &str, stations: &[u32]) -> TtTrip {
    TtTrip {
        title: title.to_string(),
        line: line.to_string(),
        stations: stations
            .iter()
            .map(|&id| TtStation {
                id,
                ..TtStation::default()
            })
            .collect(),
        ..TtTrip::default()
    }
}

fn run(name: &str) -> TtTourTrip {
    TtTourTrip {
        trip: name.to_string(),
        ..TtTourTrip::default()
    }
}

fn timetables() -> Timetables {
    let stops = vec![stop(1, "Markt"), stop(2, ""), stop(4, "Ende")];
    let tour = TtTour {
        name: "T".to_string(),
        trips: vec![run("A"), run("A"), run("BF_HOF"), run("MISSING")],
        ..TtTour::default()
    };
    let lines = vec![(
        "13".to_string(),
        TtLine {
            tours: vec![tour],
            ..TtLine::default()
        },
    )];
    let trips = vec![
        ("A".to_string(), trip("Zoo", "13N", &[1, 2, 4, 1])),
        ("BF_HOF".to_string(), trip("Depot", "", &[1])),
    ];
    Timetables::from_parts(stops, lines, trips)
}

#[test]
fn a_named_stop_gets_the_trip_weighted_by_its_runs() {
    let table = destination_table(&timetables());
    assert_eq!(table.options(1), &[(Destination::new("13N", "Zoo"), 2)]);
    assert_eq!(table.options(4), &[(Destination::new("13N", "Zoo"), 2)]);
}

#[test]
fn an_unnamed_or_unlisted_stop_serves_nothing() {
    let table = destination_table(&timetables());
    assert!(table.options(2).is_empty());
    assert!(table.options(9).is_empty());
}

#[test]
fn depot_runs_are_left_out() {
    assert!(is_depot_run("BF_HOFS-UASP"));
    assert!(is_depot_run("bf_x"));
    assert!(!is_depot_run("13N_Zoo"));
}
