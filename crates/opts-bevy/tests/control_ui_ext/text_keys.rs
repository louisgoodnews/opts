//! Text fields, key presses and hover.

use std::sync::atomic::Ordering;

use opts_cli::protocol::code;
use serde_json::json;

use super::fixture::{fixture, rpc, settle, with_client};

#[test]
fn text_replaces_or_appends_in_a_field() {
    let mut f = fixture();
    let (a, b, c_) = with_client(&mut f.app, f.port, |c| {
        let a = c
            .call("ui.text", json!({"id": "opt.search", "text": "gla"}))
            .expect("text");
        settle(60);
        let b = c
            .call(
                "ui.text",
                json!({"id": "opt.search", "text": "sgow", "replace": false}),
            )
            .expect("append");
        settle(60);
        let c_ = c
            .call("ui.text", json!({"id": "opt.search", "text": "x"}))
            .expect("replace");
        (a, b, c_)
    });
    assert_eq!(a["value"], "gla");
    assert_eq!(b["value"], "glasgow");
    assert_eq!(c_["value"], "x");
}

#[test]
fn key_presses_modifiers_with_the_key() {
    let mut f = fixture();
    let ok = with_client(&mut f.app, f.port, |c| {
        c.call(
            "ui.key",
            json!({"key": "F", "ctrl": true, "duration_ms": 120}),
        )
        .expect("key")
    });
    assert_eq!(ok["modifiers"], json!(["ctrl"]));
    assert!(
        f.chord.load(Ordering::SeqCst),
        "Ctrl and F were held in the same frame"
    );
    let bad = with_client(&mut f.app, f.port, |c| {
        c.call("ui.key", json!({"key": "Hyper"}))
            .expect_err("bad key")
    });
    assert_eq!(rpc(bad).0, code::INVALID_PARAMS);
}

#[test]
fn hover_moves_the_pointer_to_the_element() {
    let mut f = fixture();
    let r = with_client(&mut f.app, f.port, |c| {
        c.call("ui.hover", json!({"id": "row.0"})).expect("hover")
    });
    assert_eq!(r["ok"], true);
    assert!(r["pos"][0].as_f64().unwrap() > 0.0);
    let missing = with_client(&mut f.app, f.port, |c| {
        c.call("ui.hover", json!({"id": "nope"}))
            .expect_err("missing")
    });
    assert_eq!(rpc(missing).0, code::NOT_FOUND);
}
