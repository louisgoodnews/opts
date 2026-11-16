use super::*;

#[test]
fn the_timetables_are_listed_by_line_tour_and_run() {
    let (mut app, _) = app(IBIS);
    let l = call(&mut app, "bus.timetables", json!({})).expect("lines");
    assert_eq!(l["available"], true);
    assert_eq!(
        l["lines"].as_array().expect("array").len(),
        1,
        "only the lines the player may drive"
    );
    assert_eq!(l["lines"][0]["file"], "33");
    assert_eq!(l["lines"][0]["runs"], 1);
    let all = call(&mut app, "bus.timetables", json!({ "all": true })).expect("all");
    assert_eq!(all["lines"].as_array().expect("array").len(), 2);
    let t = call(&mut app, "bus.timetables", json!({ "line": "33" })).expect("tours");
    assert_eq!(t["tours"][0]["name"], "T1");
    let r = call(
        &mut app,
        "bus.timetables",
        json!({ "line": "33", "tour": 0 }),
    )
    .expect("runs");
    assert_eq!(r["runs"][0]["start"], "08:00:00");
    assert_eq!(r["runs"][0]["title"], "Vaals");
    assert_eq!(r["runs"][0]["stops"], 2);
    let bad = call(&mut app, "bus.timetables", json!({ "line": "nope" })).expect_err("unknown");
    assert_eq!(bad.code, -32602);
}

#[test]
fn following_a_run_selects_the_route_and_the_stop_moves_the_delay() {
    let (mut app, e) = app(IBIS);
    let r = call(
        &mut app,
        "bus.set_timetable",
        json!({ "line": "33", "tour": 0, "run": 0 }),
    )
    .expect("follow");
    assert_eq!(r["schedule"]["title"], "Vaals");
    assert_eq!(r["schedule"]["stops"][1]["name"], "Bushof");
    assert_eq!(r["schedule"]["stops"][1]["arrival"], "08:06:00");
    assert_eq!(
        r["route_code"], 3301,
        "the depot's route of line 33 to Vaals"
    );
    assert_eq!(r["typing"], true, "its IBIS is keyed");
    for _ in 0..40 {
        app.update();
    }
    assert_eq!(var(&app, e, "course"), 1.0);
    let g = call(&mut app, "bus.get_route", json!({})).expect("get");
    assert_eq!(g["timetable"]["trip"], "33_V");
    let s = call(&mut app, "bus.set_stop", json!({ "index": 1 })).expect("stop");
    assert_eq!(s["stop_count"], 2);
    let past = call(&mut app, "bus.set_stop", json!({ "index": 5 })).expect_err("beyond");
    assert_eq!(past.code, -32602);
    let missing = call(
        &mut app,
        "bus.set_timetable",
        json!({ "line": "33", "run": 9 }),
    )
    .expect_err("no such run");
    assert_eq!(missing.code, -32602);
    call(&mut app, "bus.set_timetable", json!({ "clear": true })).expect("clear");
    let g = call(&mut app, "bus.get_route", json!({})).expect("get");
    assert_eq!(g["timetable"], Value::Null);
}
