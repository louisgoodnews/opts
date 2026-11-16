//! Integration tests for route selection: readiness, listing routes and keyed selection on the keypad.
use super::*;

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
