//! `bus.routes`, `bus.get_route`, `bus.set_route` (M-0099): the depot's routes are listed, a
//! selection is keyed on the vehicle's IBIS one key per step and handed to the callbacks.

use std::sync::Arc;

use bevy::prelude::*;
use opts_bevy::control::channel::dispatch;
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_bevy::route::{route_typing_system, RouteHandle, VehicleRoute};
use opts_bevy::{PlayerVehicle, VehicleController, VehicleScripts};
use opts_cli::protocol::{code, RpcError};
use opts_core::formats::hof::HofFile;
use opts_core::formats::timetable::{parse_busstops, parse_line, parse_trip, Timetables};
use opts_sim::script::ScriptEngine;
use serde_json::{json, Value};

const HOF: &str = "
[name]
Demo

[servicetrip]
Betriebsfahrt

[global_strings]
2
a
b

stringcount_terminus
2

[addterminus_list]
	11	Vaals	VAALS
	22	Bushof	BUSHOF
[end]

stringcount_busstop
2

[addbusstop_list]
Ponttor	PONTTOR
Bushof	BUSHOF
[end]

[infosystem_trip]
3301
FUCHSERDE - VAALS
11
33

[infosystem_busstop_list]
2
Bushof
Ponttor

[infosystem_trip]
3302
VAALS - FUCHSERDE
22
33

[infosystem_busstop_list]
1
Ponttor
";

/// A keypad that records what is pressed: the digits typed since the last mode key, the route
/// entered with `ibis_eingabe` in mode 1 (line and suffix), 2 (course) and 3 (destination).
const IBIS: &str = "
{trigger:ibis_setmode_linie_kurs}\n1 (S.L.mode)\n0 (S.L.input)\n{end}
{trigger:ibis_setmode_route}\n2 (S.L.mode)\n0 (S.L.input)\n{end}
{trigger:ibis_setmode_ziel}\n3 (S.L.mode)\n0 (S.L.input)\n{end}
{trigger:ibis_eingabe}\n(L.L.mode) 1 =\n{if}\n(L.L.input) (S.L.line_complex)\n{endif}\n(L.L.mode) 2 =\n{if}\n(L.L.input) (S.L.course)\n{endif}\n(L.L.mode) 3 =\n{if}\n(L.L.input) (S.L.terminus)\n{endif}\n{end}
{trigger:ibis_0}\n(L.L.input) 10 * 0 + (S.L.input)\n{end}
{trigger:ibis_1}\n(L.L.input) 10 * 1 + (S.L.input)\n{end}
{trigger:ibis_2}\n(L.L.input) 10 * 2 + (S.L.input)\n{end}
{trigger:ibis_3}\n(L.L.input) 10 * 3 + (S.L.input)\n{end}
{trigger:ibis_4}\n(L.L.input) 10 * 4 + (S.L.input)\n{end}
{trigger:ibis_5}\n(L.L.input) 10 * 5 + (S.L.input)\n{end}
{trigger:ibis_6}\n(L.L.input) 10 * 6 + (S.L.input)\n{end}
{trigger:ibis_7}\n(L.L.input) 10 * 7 + (S.L.input)\n{end}
{trigger:ibis_8}\n(L.L.input) 10 * 8 + (S.L.input)\n{end}
{trigger:ibis_9}\n(L.L.input) 10 * 9 + (S.L.input)\n{end}
";

fn timetables() -> Timetables {
    Timetables::from_parts(
        parse_busstops(b"[busstop]\nPonttor\n1\n100\n0\n0\n\n[busstop]\nBushof\n1\n101\n0\n0\n\n"),
        vec![
            (
                "33".into(),
                parse_line(b"[userallowed]\n[priority]\n1\n[newtour]\nT1\nBus\n1\n[addtrip]\n33_V\n0\n480.000\n"),
            ),
            ("depot".into(), parse_line(b"[priority]\n1\n[newtour]\nD\nBus\n1\n")),
        ],
        vec![(
            "33_V".into(),
            parse_trip(b"[trip]\n\nVaals\n33\n[station_typ2]\n100\n[station_typ2]\n101\n[profile]\nstandard\n6.0\n[profile_man_arr_time]\n0\n0.0\n[profile_man_arr_time]\n1\n6.0\n"),
        )],
    )
}

fn app(script: &str) -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let mut registry = ControlRegistry::default();
    opts_game::control_route::register(&mut registry);
    app.insert_resource(registry);
    app.add_systems(Update, route_typing_system);
    let (mut engine, d) = ScriptEngine::from_sources(&[("t.osc", script)]);
    assert!(d.is_empty(), "{d:?}");
    engine.run_init();
    let hof = Arc::new(HofFile::parse_str(HOF).expect("hof"));
    let handle = RouteHandle::new(Some(hof));
    handle.set_timetables(Some(Arc::new(timetables())));
    let e = app
        .world_mut()
        .spawn((
            PlayerVehicle,
            VehicleController::standard_bus(11_500.0, 5.85),
            VehicleScripts::new(engine),
            VehicleRoute::new(handle),
        ))
        .id();
    (app, e)
}

fn call(app: &mut App, cmd: &str, p: Value) -> Result<Value, RpcError> {
    match dispatch(app.world_mut(), cmd, &p) {
        Outcome::Done(r) => r,
        Outcome::Wait(_) => panic!("answers at once"),
    }
}

fn var(app: &App, e: Entity, name: &str) -> f32 {
    let s = app.world().get::<VehicleScripts>(e).expect("scripts");
    let v = s.engine.lock().expect("engine").get(name);
    v
}

#[test]
fn without_a_player_vehicle_it_is_not_ready() {
    let mut app = App::new();
    let mut registry = ControlRegistry::default();
    opts_game::control_route::register(&mut registry);
    app.insert_resource(registry);
    for c in ["bus.routes", "bus.get_route"] {
        assert_eq!(
            call(&mut app, c, json!({})).expect_err("no vehicle").code,
            code::NOT_READY
        );
    }
}

#[test]
fn the_routes_of_the_depot_are_listed_with_their_destination() {
    let (mut app, _) = app(IBIS);
    let r = call(&mut app, "bus.routes", json!({})).expect("list");
    assert_eq!(r["depot"], "Demo");
    assert_eq!(r["count"], 2);
    assert_eq!(r["routes"][0]["route_code"], 3301);
    assert_eq!(r["routes"][0]["line"], 33);
    assert_eq!(r["routes"][0]["course"], 1);
    assert_eq!(r["routes"][0]["terminus"], "VAALS");
    assert_eq!(r["selected"], Value::Null);
    let one = call(&mut app, "bus.routes", json!({ "limit": 1 })).expect("limit");
    assert_eq!(one["routes"].as_array().expect("array").len(), 1);
}

#[test]
fn a_selection_is_keyed_on_the_ibis_one_key_per_step() {
    let (mut app, e) = app(IBIS);
    let r = call(
        &mut app,
        "bus.set_route",
        json!({ "line": 33, "course": 2 }),
    )
    .expect("set");
    assert_eq!(r["selected"]["route_code"], 3302);
    assert_eq!(r["keypad"], true);
    assert_eq!(r["typing"], true);
    assert_eq!(var(&app, e, "line_complex"), 0.0, "nothing pressed yet");
    for _ in 0..40 {
        app.update();
    }
    // Line 33 and suffix 0 as `3300`, course `2`, destination code `22`.
    assert_eq!(var(&app, e, "line_complex"), 3300.0);
    assert_eq!(var(&app, e, "course"), 2.0);
    assert_eq!(var(&app, e, "terminus"), 22.0);
    let g = call(&mut app, "bus.get_route", json!({})).expect("get");
    assert_eq!(g["selected"]["route_code"], 3302);
    assert_eq!(g["typing"], false);
}

#[test]
fn an_unknown_route_is_refused_and_a_clear_cancels() {
    let (mut app, _) = app(IBIS);
    let err = call(&mut app, "bus.set_route", json!({ "route_code": 99999 })).expect_err("unknown");
    assert_eq!(err.code, -32602);
    call(&mut app, "bus.set_route", json!({ "route_code": 3301 })).expect("set");
    call(&mut app, "bus.set_route", json!({ "clear": true })).expect("clear");
    let g = call(&mut app, "bus.get_route", json!({})).expect("get");
    assert_eq!(g["selected"], Value::Null);
    let bad = call(&mut app, "bus.set_route", json!({})).expect_err("nothing given");
    assert_eq!(bad.code, -32602);
}

#[test]
fn a_vehicle_without_a_keypad_still_takes_the_route() {
    let (mut app, _) = app("{frame}\n1 (S.L.x)\n{end}");
    let r = call(&mut app, "bus.set_route", json!({ "route_code": 3301 })).expect("set");
    assert_eq!(r["keypad"], false);
    assert_eq!(r["typing"], false);
    assert_eq!(r["selected"]["route_code"], 3301);
}

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
