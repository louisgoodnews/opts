//! The timetable callbacks answered for a followed run.

use super::*;

#[test]
fn a_followed_run_answers_the_timetable_callbacks_and_selects_the_depot_route() {
    let route = handle();
    let (schedule, code) = route.select_run("33", 0, 0).expect("run");
    assert_eq!(schedule.stops.len(), 3);
    assert_eq!(
        code,
        Some(3301),
        "the route of line 33 to the terminus `Vaals`"
    );
    assert_eq!(route.selection().expect("selected").route_code, 3301);
    let mut h = vehicle_callbacks_with_context(VehicleHostContext {
        route: route.clone(),
        ..Default::default()
    });
    assert_eq!(call(&mut h, "getttbusstopcount", &[]).numbers, [3.0]);
    assert_eq!(call(&mut h, "getttbusstopname", &[1.0]).strings, ["Bushof"]);
    assert_eq!(call(&mut h, "getttlinestring", &[]).strings, ["33"]);
    assert_eq!(call(&mut h, "getttterminusindex", &[]).numbers, [0.0]);
    // Start 08:00 = 28,800 s; stop 1 arrives 4 minutes later and, with no dwell, leaves then.
    assert_eq!(
        call(&mut h, "getttbusstoparr", &[1.0]).numbers,
        [28_800.0 + 240.0]
    );
    assert_eq!(
        call(&mut h, "getttbusstopdep", &[1.0]).numbers,
        [28_800.0 + 240.0]
    );
    assert_eq!(call(&mut h, "getttbusstopdep", &[0.0]).numbers, [28_800.0]);
    assert_eq!(
        call(&mut h, "getttbusstoparr", &[9.0]).numbers,
        [0.0],
        "beyond the stops"
    );
}

#[test]
fn unknown_runs_and_maps_without_timetables_are_refused() {
    let route = handle();
    assert!(matches!(
        route.select_run("33", 0, 5),
        Err(opts_bevy::route::RouteError::UnknownRun(_))
    ));
    assert!(matches!(
        handle_with_depot().select_run("33", 0, 0),
        Err(opts_bevy::route::RouteError::NoTimetables)
    ));
}
