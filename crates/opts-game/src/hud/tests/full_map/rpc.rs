//! Full-size map tests: the protocol opens and moves the map and refuses bad values.

use super::*;

#[test]
fn the_protocol_opens_and_moves_the_map_and_refuses_bad_values() {
    use serde_json::json;
    let mut app = map_app();
    let call = |app: &mut App, params: serde_json::Value| match protocol::fullmap(
        app.world_mut(),
        &params,
    ) {
        opts_bevy::control::state::Outcome::Done(r) => r,
        _ => panic!("not immediate"),
    };
    let v = call(
        &mut app,
        json!({"open": true, "span": 2500.0, "x": 10.0, "y": -40.0}),
    )
    .expect("ok");
    assert_eq!(v["span_m"], 2500.0);
    assert_eq!(v["follow"], false);
    run(&mut app, 3);
    let s = full(&app);
    assert!(s.open, "the request opened it");
    // The view that was set is kept; it is not re-centred on the vehicle.
    assert_eq!(s.viewport.span_m, 2500.0);
    assert!((s.viewport.centre.x - 10.0).abs() < 1e-9 && (s.viewport.centre.y + 40.0).abs() < 1e-9);
    let read = call(&mut app, json!({})).expect("read");
    assert_eq!(read["open"], true);
    assert!(read["redraws"].as_u64().expect("n") > 0 && read["min_span_m"] == 50.0);
    for bad in [
        json!({"span": 10.0}),
        json!({"span": 50000.0}),
        json!({"x": 1.0}),
    ] {
        assert!(call(&mut app, bad.clone()).is_err(), "{bad}");
    }
    let closed = call(&mut app, json!({"open": false})).expect("close");
    assert!(closed["open"].is_boolean());
    run(&mut app, 2);
    assert!(!full(&app).open);
}
