//! Tests of the route page.

use super::route_page::{route_view, TimetableView};
use super::*;
use opts_core::formats::hof::HofFile;
use opts_core::formats::timetable::{parse_busstops, parse_line, parse_trip, Timetables};
use std::sync::Arc;

const HOF: &str = "
[name]
Demo

[servicetrip]
x

[global_strings]
1
a

stringcount_terminus
1

[addterminus_list]
	11	Vaals	VAALS
	22	Bushof	BUSHOF
[end]

stringcount_busstop
1

[addbusstop_list]
Ponttor
[end]

[infosystem_trip]
3301
A - B
11
33

[infosystem_busstop_list]
1
Ponttor

[infosystem_trip]
5002
C - D
22
50

[infosystem_busstop_list]
1
Ponttor
";

pub(super) fn handle() -> opts_bevy::route::RouteHandle {
    let h =
        opts_bevy::route::RouteHandle::new(Some(Arc::new(HofFile::parse_str(HOF).expect("hof"))));
    h.set_timetables(Some(Arc::new(Timetables::from_parts(
        parse_busstops(b"[busstop]\nPonttor\n1\n100\n0\n0\n\n"),
        vec![(
            "33".into(),
            parse_line(b"[userallowed]\n[priority]\n1\n[newtour]\nT1\nBus\n1\n[addtrip]\n33_V\n0\n480.000\n"),
        )],
        vec![(
            "33_V".into(),
            parse_trip(b"[trip]\n\nVAALS\n33\n[station_typ2]\n100\n[profile]\nstandard\n1.0\n"),
        )],
    ))));
    h
}

#[test]
fn the_depot_routes_are_listed_with_line_course_and_destination_and_the_search_narrows_them() {
    let mut menu = VehicleMenu::default();
    let h = handle();
    let v = route_view(&menu, &h);
    assert_eq!(v.total_routes, 2);
    assert_eq!(v.routes[0], (3301, "33/01  VAALS".to_string()));
    assert_eq!(v.routes[1].1, "50/02  BUSHOF");
    assert_eq!(v.summary, "No route selected (free driving).");
    menu.route_search = "bush".into();
    let v = route_view(&menu, &h);
    assert_eq!(v.routes.len(), 1);
    assert_eq!(v.total_routes, 2, "the count is of the whole depot");
    h.select(3301).expect("select");
    let v = route_view(&menu, &h);
    assert_eq!(v.selected, Some(3301));
    assert!(v.summary.starts_with("Route: 33/01"), "{}", v.summary);
}

#[test]
fn the_timetable_column_goes_from_lines_to_tours_to_runs() {
    let mut menu = VehicleMenu::default();
    let h = handle();
    let Some(TimetableView::Lines(l)) = route_view(&menu, &h).timetable else {
        panic!("lines first");
    };
    assert_eq!(l, vec![(0, "33  (1 tours)".to_string())]);
    menu.tt_line = Some(0);
    let Some(TimetableView::Tours(name, t)) = route_view(&menu, &h).timetable else {
        panic!("then the tours");
    };
    assert_eq!(name, "33");
    assert_eq!(t[0].1, "T1  ·  Bus  (1 runs)");
    menu.tt_tour = Some(0);
    let Some(TimetableView::Runs(_, r)) = route_view(&menu, &h).timetable else {
        panic!("then the runs");
    };
    assert_eq!(r[0].1, "08:00  33  VAALS");
    h.select_run("33", 0, 0).expect("run");
    assert!(
        route_view(&menu, &h)
            .summary
            .starts_with("Timetable: 33 · T1 33 → VAALS, start 08:00"),
        "{}",
        route_view(&menu, &h).summary
    );
}

#[test]
fn a_map_without_timetables_has_no_right_column() {
    let h =
        opts_bevy::route::RouteHandle::new(Some(Arc::new(HofFile::parse_str(HOF).expect("hof"))));
    assert!(route_view(&VehicleMenu::default(), &h).timetable.is_none());
}
