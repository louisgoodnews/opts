//! The route report and its helpers.

use super::phases::Loop;
use super::*;

/// System values the engine itself sets in every `run_frame` (`GetTime`, `Timegap`): never unfed.
pub(super) const ENGINE_SET: &[&str] = &["gettime", "timegap"];

pub(super) fn find_ci(base: &Path, rel: &str) -> Option<PathBuf> {
    let mut cur = base.to_path_buf();
    for comp in rel
        .split(['\\', '/'])
        .filter(|c| !c.is_empty() && *c != ".")
    {
        if comp == ".." {
            cur = cur.parent()?.to_path_buf();
            continue;
        }
        let direct = cur.join(comp);
        if direct.exists() {
            cur = direct;
            continue;
        }
        let hit = std::fs::read_dir(&cur)
            .ok()?
            .flatten()
            .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(comp))?;
        cur = hit.path();
    }
    Some(cur)
}

pub(super) fn buses(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() {
            buses(&p, out);
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("bus")) {
            out.push(p);
        }
    }
}

/// A stable seed from the vehicle's path (FNV-1a).
pub(super) fn seed_of(path: &str) -> u64 {
    path.bytes().fold(0xCBF2_9CE4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01B3)
    })
}

/// The output rows the scripts wrote that OPTS does not read yet.
pub(super) fn unconsumed(writes: &BTreeSet<String>) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for w in writes {
        for row in OUTPUT_TABLE {
            let hit = if row.name.ends_with('_') {
                w.starts_with(row.name)
            } else {
                w == row.name
            };
            if hit && !row.consumed {
                out.insert(row.name.to_owned());
            }
        }
    }
    out
}

/// The depot of a vehicle: a `.hof` of its folder whose name contains `filter`, else the game's
/// rule ([`opts_bevy::route::find_depot`]).
pub(super) fn depot_for(
    dir: &Path,
    root: &Path,
    filter: Option<&str>,
) -> Option<std::sync::Arc<opts_core::formats::hof::HofFile>> {
    let pick = filter.and_then(|f| {
        let mut files: Vec<_> = std::fs::read_dir(dir)
            .ok()?
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.extension().is_some_and(|e| e.eq_ignore_ascii_case("hof"))
                    && p.file_name()
                        .is_some_and(|n| n.to_string_lossy().to_lowercase().contains(f))
            })
            .collect();
        files.sort();
        files.iter().find_map(|p| {
            opts_core::formats::hof::HofFile::parse_bytes(&std::fs::read(p).ok()?).ok()
        })
    });
    pick.or_else(|| opts_bevy::route::find_depot(dir, root, &[]))
        .map(std::sync::Arc::new)
}

/// Selects route `code` on a started vehicle the way a driver does (the IBIS keys) and reports
/// what the scripts then hold: the selection, the keys, the strings the depot defines for it and
/// whether the scripts' own string variables carry them (M-0099).
pub(super) fn route_report(
    bus: &BusManifest,
    dir: &Path,
    root: &Path,
    depot: Option<std::sync::Arc<opts_core::formats::hof::HofFile>>,
    code: i32,
    seed: u64,
    timetable: Option<(&(String, usize, usize), PathBuf)>,
) -> serde_json::Value {
    let route = opts_bevy::route::RouteHandle::new(depot.clone());
    if let Some((_, map_dir)) = &timetable {
        route.set_timetables(opts_bevy::route::find_timetables(map_dir).map(std::sync::Arc::new));
    }
    let (mut engine, _) =
        load_vehicle_scripts(bus, |r| find_ci(dir, r).or_else(|| find_ci(root, r)));
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
    let before = lp.engine.get("target_index_int");
    let mut schedule_json = serde_json::Value::Null;
    let selection = if let Some(((line, tour, run), _)) = &timetable {
        match route.select_run(line, *tour, *run) {
            Ok((sc, Some(_))) => {
                schedule_json = serde_json::json!({
                    "tour": sc.tour, "trip": sc.trip, "title": sc.title, "line": sc.line,
                    "start_s": sc.start_s, "stops": sc.stops.len(),
                    "first": sc.stops.iter().take(3).map(|x| (x.name.clone(), x.arrival_s, x.departure_s)).collect::<Vec<_>>(),
                });
                route.selection().expect("the route was selected")
            }
            Ok((sc, None)) => {
                return serde_json::json!({ "error": "the depot has no route for this run", "title": sc.title, "line": sc.line });
            }
            Err(e) => return serde_json::json!({ "error": e.to_string() }),
        }
    } else {
        match route.select(code) {
            Ok(s) => s,
            Err(e) => return serde_json::json!({ "error": e.to_string() }),
        }
    };
    let keypad = lp.keys.type_selection(&lp.engine, &selection).is_ok();
    let ziel_before = lp.engine.get("ziel_neu");
    let displays_answered = opts_bevy::route::schedule_target(&mut lp.engine, &route);
    lp.pending_target_s = opts_bevy::route::TARGET_OFFER_S;
    let ziel_after = lp.engine.get("ziel_neu");
    eprintln!(
        "ziel_neu before {ziel_before} after {ziel_after} timer {} ai {} line_anim {}",
        lp.engine.get("matrix_ziel_animationtimer"),
        lp.engine.get("ai"),
        lp.engine.get("matrix_linie_animationtimer")
    );
    lp.run(150.0);
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
    let carries = |wanted: &[String]| {
        held.iter()
            .filter(|(_, v)| {
                // Displays pad and spread their text: compare without spaces, by containment.
                let flat = |t: &str| t.chars().filter(|c| !c.is_whitespace()).collect::<String>();
                let have = flat(v);
                wanted.iter().any(|w| {
                    let w = flat(w);
                    w.chars().count() >= 3 && have.contains(&w)
                })
            })
            .map(|(n, _)| n.clone())
            .collect::<Vec<_>>()
    };
    let terminus_strings = terminus.as_ref().map(|t| t.1.clone()).unwrap_or_default();
    let extra: serde_json::Map<String, serde_json::Value> = std::env::var("RIG_VARS")
        .map(|l| {
            l.split(',')
                .map(|n| {
                    (
                        n.to_string(),
                        serde_json::json!(lp.engine.get(&n.to_lowercase())),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
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
        "variables_carrying_terminus_strings": carries(&terminus_strings),
        "variables_carrying_stop_strings": carries(&stop_strings),
    })
}
