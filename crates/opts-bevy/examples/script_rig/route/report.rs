//! The route report: starts a vehicle loop wired to the route, selects the run or route and
//! reports what the scripts hold afterwards.

use super::phases::Loop;
use super::*;

/// The route handle and a started vehicle loop wired to it; `dirs` is (vehicle dir, OMSI root).
fn start_route_loop(
    bus: &BusManifest,
    dirs: (&Path, &Path),
    depot: &Depot,
    seed: u64,
    timetable: &Timetable<'_>,
) -> (opts_bevy::route::RouteHandle, Loop) {
    let (dir, root) = dirs;
    let route = opts_bevy::route::RouteHandle::new(depot.clone());
    if let Some((_, map_dir)) = timetable {
        route.set_timetables(opts_bevy::route::find_timetables(map_dir).map(std::sync::Arc::new));
    }
    let (mut engine, _) = load_vehicle_scripts(bus, |r| {
        read_script(find_ci(dir, r).or_else(|| find_ci(root, r)))
    });
    engine.seed_random(seed as u32);
    engine.set_external_handler(opts_bevy::host_callbacks::vehicle_callbacks_with_context(
        opts_bevy::host_callbacks::VehicleHostContext {
            seed,
            route: route.clone(),
            ..Default::default()
        },
    ));
    opts_bevy::route::apply_to_scripts(&mut engine, &route);
    let mut lp = Loop::new(bus, None, engine, 1.0 / 60.0);
    lp.route = Some(route.clone());
    lp.ready_basic();
    (route, lp)
}

/// Selects the run (timetable) or the route `code`; the schedule as JSON, or the error report.
fn select_route(
    route: &opts_bevy::route::RouteHandle,
    timetable: &Timetable<'_>,
    code: i32,
) -> Result<(opts_bevy::route::RouteSelection, serde_json::Value), serde_json::Value> {
    if let Some(((line, tour, run), _)) = timetable {
        match route.select_run(line, *tour, *run) {
            Ok((sc, Some(_))) => {
                let schedule_json = serde_json::json!({
                    "tour": sc.tour, "trip": sc.trip, "title": sc.title, "line": sc.line,
                    "start_s": sc.start_s, "stops": sc.stops.len(),
                    "first": sc.stops.iter().take(3).map(|x| (x.name.clone(), x.arrival_s, x.departure_s)).collect::<Vec<_>>(),
                });
                let selection = route.selection().expect("the route was selected");
                Ok((selection, schedule_json))
            }
            Ok((sc, None)) => Err(
                serde_json::json!({ "error": "the depot has no route for this run", "title": sc.title, "line": sc.line }),
            ),
            Err(e) => Err(serde_json::json!({ "error": e.to_string() })),
        }
    } else {
        match route.select(code) {
            Ok(s) => Ok((s, serde_json::Value::Null)),
            Err(e) => Err(serde_json::json!({ "error": e.to_string() })),
        }
    }
}

/// Types the selection on the keypad, schedules the target display and runs the vehicle for
/// 150 s; returns (keypad accepted, displays answered).
fn drive_selection(
    lp: &mut Loop,
    route: &opts_bevy::route::RouteHandle,
    selection: &opts_bevy::route::RouteSelection,
) -> (bool, bool) {
    let keypad = lp.keys.type_selection(&lp.engine, selection).is_ok();
    let ziel_before = lp.engine.get("ziel_neu");
    let displays_answered = opts_bevy::route::schedule_target(&mut lp.engine, route);
    lp.pending_target_s = opts_bevy::route::TARGET_OFFER_S;
    let ziel_after = lp.engine.get("ziel_neu");
    eprintln!(
        "ziel_neu before {ziel_before} after {ziel_after} timer {} ai {} line_anim {}",
        lp.engine.get("matrix_ziel_animationtimer"),
        lp.engine.get("ai"),
        lp.engine.get("matrix_linie_animationtimer")
    );
    lp.run(150.0);
    (keypad, displays_answered)
}

/// Selects route `code` on a started vehicle the way a driver does (the IBIS keys) and reports
/// what the scripts then hold: the selection, the keys, the strings the depot defines for it and
/// whether the scripts' own string variables carry them (M-0099).
pub(in super::super) fn route_report(
    bus: &BusManifest,
    dir: &Path,
    root: &Path,
    depot: Depot,
    code: i32,
    seed: u64,
    timetable: Timetable<'_>,
) -> serde_json::Value {
    let (route, mut lp) = start_route_loop(bus, (dir, root), &depot, seed, &timetable);
    let before = lp.engine.get("target_index_int");
    let (selection, schedule_json) = match select_route(&route, &timetable, code) {
        Ok(picked) => picked,
        Err(report) => return report,
    };
    let (keypad, displays_answered) = drive_selection(&mut lp, &route, &selection);
    let hof = depot.expect("selected a route, so there is a depot");
    let terminus = hof
        .find_terminus_by_code(selection.terminus_code)
        .map(|(i, t)| (i, t.strings.clone()));
    let first_stop = route.with(|s| s.route().and_then(|r| r.busstops.first().cloned()));
    let stop_strings: Vec<String> = first_stop
        .as_deref()
        .and_then(|id| hof.find_busstop_by_ident(id))
        .map(|(_, b)| b.strings.clone())
        .unwrap_or_default();
    let held: Vec<(String, String)> = lp
        .engine
        .string_variables()
        .into_iter()
        .filter(|(_, v)| !v.is_empty())
        .collect();
    let terminus_strings = terminus.as_ref().map(|t| t.1.clone()).unwrap_or_default();
    let extra = extra_vars(&lp);
    serde_json::json!({
        "extra": extra,
        "yard": lp.engine.get_string("yard"),
        "setlineto": lp.engine.get_string("setlineto"),
        "keypad": keypad,
        "schedule": schedule_json,
        "displays_answered": displays_answered,
        "route_code": selection.route_code,
        "line": selection.line,
        "course": selection.course,
        "terminus_code": selection.terminus_code,
        "terminus_index": terminus.as_ref().map(|t| t.0),
        "target_index_int": lp.engine.get("target_index_int"),
        "target_index_int_before": before,
        "ibis_route": lp.engine.get("ibis_route"),
        "ibis_terminusindex": lp.engine.get("ibis_terminusindex"),
        "matrix_terminusact": lp.engine.get("matrix_terminusact"),
        "matrix_steckschild_termindex": lp.engine.get("matrix_steckschild_termindex"),
        "ibis_mode": lp.engine.get("ibis_mode"),
        "rlbnd_ziel": lp.engine.get("rlbnd_ziel"),
        "rlbnd_ziel_target": lp.engine.get("rlbnd_ziel_target"),
        "ibis_terminuscode": lp.engine.get("ibis_terminuscode"),
        "act_busstop": lp.engine.get_string("act_busstop"),
        "expected_first_stop": first_stop,
        "terminus_strings": terminus_strings,
        "variables_carrying_terminus_strings": carrying(&held, &terminus_strings),
        "variables_carrying_stop_strings": carrying(&held, &stop_strings),
    })
}
