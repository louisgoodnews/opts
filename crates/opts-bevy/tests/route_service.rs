//! The route service (M-0099): selection, what the host feeds from it every frame, and the
//! repeated offer of the target to a display that is busy.

use std::sync::Arc;

use opts_bevy::route::{
    feed_route, keep_target, schedule_target, RouteError, RouteHandle, TARGET_OFFER_S,
};
use opts_core::formats::hof::HofFile;
use opts_sim::script::inputs::HostInputs;
use opts_sim::script::ScriptEngine;

const HOF: &str = "
[name]
Depot

[servicetrip]
x

[global_strings]
1
a

stringcount_terminus
1

[addterminus_list]
	11	Vaals
	22	Bushof
[end]

stringcount_busstop
1

[addbusstop_list]
Ponttor
[end]

[infosystem_trip]
3301
A - B
22
XXX

[infosystem_busstop_list]
1
Ponttor
";

fn handle() -> RouteHandle {
    RouteHandle::new(Some(Arc::new(HofFile::parse_str(HOF).expect("hof"))))
}

#[test]
fn selection_changes_what_the_routes_say_and_unknown_routes_are_refused() {
    let h = handle();
    assert_eq!(
        h.select(1).expect_err("unknown"),
        RouteError::UnknownRoute(1)
    );
    assert!(h.selection().is_none());
    let s = h.select(3301).expect("known");
    assert_eq!((s.line, s.course, s.terminus_code), (33, 1, 22));
    // A depot that marks "no line text" with X shows the line number.
    assert_eq!(s.line_string, "33");
    h.clear();
    assert!(h.selection().is_none());
    assert_eq!(
        RouteHandle::new(None).select(3301).expect_err("no depot"),
        RouteError::NoDepot
    );
}

#[test]
fn the_feed_carries_the_terminus_row_and_the_schedule_flag() {
    let h = handle();
    let mut inputs = HostInputs::default();
    feed_route(&mut inputs, &h);
    assert_eq!(inputs.ai_target_index, -1.0);
    assert!(!inputs.schedule_active);
    h.select(3301).expect("known");
    feed_route(&mut inputs, &h);
    assert_eq!(inputs.ai_target_index, 1.0, "terminus 22 is the second row");
    assert!(inputs.schedule_active);
}

/// A display that ignores the trigger for its first three frames (switching on).
const BUSY_DISPLAY: &str = "
{frame}
(L.L.age) 1 + (S.L.age)
{end}
{trigger:ai_scheduled_settarget}
(L.L.age) 3 >
{if}
(L.L.ai_target_index) (S.L.target_index_int)
{endif}
{end}
";

#[test]
fn the_target_is_offered_again_until_the_display_takes_it() {
    let h = handle();
    h.select(3301).expect("known");
    let (mut engine, d) = ScriptEngine::from_sources(&[("t.osc", BUSY_DISPLAY)]);
    assert!(d.is_empty(), "{d:?}");
    engine.run_init();
    let mut pending: f32 = 0.0;
    assert!(
        schedule_target(&mut engine, &h),
        "a script answers the trigger"
    );
    pending = pending.max(TARGET_OFFER_S);
    engine.set("target_index_int", -1.0);
    for _ in 0..10 {
        let mut inputs = HostInputs::default();
        feed_route(&mut inputs, &h);
        engine.set("ai_target_index", inputs.ai_target_index);
        engine.run_frame(1.0 / 60.0, false);
        keep_target(&mut engine, &h, &mut pending, 1.0 / 60.0);
    }
    assert_eq!(
        engine.get("target_index_int"),
        1.0,
        "taken once it was ready"
    );
    assert_eq!(
        pending, 0.0,
        "the offer ends when the display shows the target"
    );
}

mod timetable {
    use super::*;
    use opts_bevy::host_callbacks::{vehicle_callbacks_with_context, VehicleHostContext};
    use opts_core::formats::timetable::{parse_busstops, parse_line, parse_trip, Timetables};
    use opts_sim::script::Stacks;

    const TTP: &str = "[trip]\n\nVaals\n33\n[station_typ2]\n100\n[station_typ2]\n101\n[station_typ2]\n102\n[profile]\nstandard\n10.000\n[profile_man_arr_time]\n0\n0.0\n[profile_man_arr_time]\n1\n4.0\n[profile_man_arr_time]\n2\n10.0\n";
    const TTL: &str =
        "[userallowed]\n[priority]\n1\n[newtour]\nT1\nBus\n1\n[addtrip]\n33_V\n0\n480.000\n";
    const STOPS: &str = "[busstop]\nPonttor\n1\n100\n0\n0\n\n[busstop]\nBushof\n1\n101\n0\n0\n\n[busstop]\nVaals\n1\n102\n0\n0\n\n";

    fn handle() -> RouteHandle {
        let h = handle_with_depot();
        h.set_timetables(Some(Arc::new(Timetables::from_parts(
            parse_busstops(STOPS.as_bytes()),
            vec![("33".into(), parse_line(TTL.as_bytes()))],
            vec![("33_V".into(), parse_trip(TTP.as_bytes()))],
        ))));
        h
    }

    fn handle_with_depot() -> RouteHandle {
        // The depot of the module above, with a terminus whose text is the trip's destination
        // and a route of line 33 to it.
        let hof = "
[name]
Depot

[servicetrip]
x

[global_strings]
1
a

stringcount_terminus
1

[addterminus_list]
	11	Vaals
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
";
        RouteHandle::new(Some(Arc::new(HofFile::parse_str(hof).expect("hof"))))
    }

    fn call(h: &mut opts_sim::script::ExternalHandler, name: &str, args: &[f32]) -> Stacks {
        let mut st = Stacks::default();
        st.numbers.extend_from_slice(args);
        assert!(h(name, &mut st), "{name}");
        st
    }

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
    fn the_delay_is_the_clock_minus_the_arrival_at_the_current_stop() {
        let route = handle();
        route.select_run("33", 0, 0).expect("run");
        let mut h = vehicle_callbacks_with_context(VehicleHostContext {
            route: route.clone(),
            ..Default::default()
        });
        route.set_clock(28_800.0 + 30.0);
        assert_eq!(
            call(&mut h, "getttdelay", &[]).numbers,
            [30.0],
            "30 s late at the first stop"
        );
        route.set_stop(1);
        route.set_clock(28_800.0 + 200.0);
        assert_eq!(
            call(&mut h, "getttdelay", &[]).numbers,
            [-40.0],
            "40 s early at stop 1"
        );
        route.clear_schedule();
        route.set_clock(40_000.0);
        assert_eq!(
            call(&mut h, "getttdelay", &[]).numbers,
            [0.0],
            "free driving has none"
        );
    }

    #[test]
    fn the_delay_wraps_at_midnight() {
        let route = handle();
        route.select_run("33", 0, 0).expect("run");
        // The run's first stop is at 08:00; a clock at 08:00 the next day is on time, 23:50 the
        // evening before is 8 h 10 min early (not 15 h 50 min late).
        route.set_clock(8.0 * 3600.0);
        assert_eq!(route.with(|s| s.delay_s), 0.0);
        route.set_clock(23.0 * 3600.0 + 50.0 * 60.0);
        assert_eq!(
            route.with(|s| s.delay_s),
            15.0 * 3600.0 + 50.0 * 60.0 - 86_400.0
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
}
