use super::*;
use serde_json::{json, Value};

#[test]
fn request_round_trips_and_omits_empty_params() {
    let r = Request::new(7, "ping", Value::Null);
    let line = serde_json::to_string(&r).unwrap();
    assert_eq!(line, r#"{"jsonrpc":"2.0","id":7,"method":"ping"}"#);
    assert_eq!(parse_line(&line).unwrap(), Incoming::Single(r));
}

#[test]
fn error_responses_carry_code_and_data() {
    let e = RpcError::new(code::NOT_FOUND, "entity 9").with_data(json!({"id": 9}));
    let resp = Response::err(json!(1), e.clone());
    let line = serde_json::to_string(&resp).unwrap();
    let back: Response = serde_json::from_str(&line).unwrap();
    // The wire form adds the symbolic name and the retry hint; the caller's data stays.
    let got = back.into_result().unwrap_err();
    assert_eq!(got, e.structured());
    let data = got.data.expect("data");
    assert_eq!(data["id"], 9);
    assert_eq!(data["name"], "NOT_FOUND");
    assert_eq!(data["retryable"], false);
    let gated = RpcError::new(code::GATED, "blocked").structured();
    assert_eq!(gated.data.expect("data")["retryable"], true);
}

#[test]
fn invalid_input_maps_to_the_right_codes() {
    assert_eq!(parse_line("{nope").unwrap_err().code, code::PARSE_ERROR);
    assert_eq!(parse_line("[]").unwrap_err().code, code::INVALID_REQUEST);
    assert_eq!(parse_line("5").unwrap_err().code, code::INVALID_REQUEST);
    let wrong = r#"{"jsonrpc":"1.0","id":1,"method":"ping"}"#;
    assert_eq!(parse_line(wrong).unwrap_err().code, code::INVALID_REQUEST);
    let positional = r#"{"jsonrpc":"2.0","id":1,"method":"x","params":[1]}"#;
    assert_eq!(
        parse_line(positional).unwrap_err().code,
        code::INVALID_PARAMS
    );
}

#[test]
fn batches_keep_per_element_errors() {
    let line = r#"[{"jsonrpc":"2.0","id":1,"method":"ping"}, 3]"#;
    let Incoming::Batch(items) = parse_line(line).unwrap() else {
        panic!("expected batch");
    };
    assert!(items[0].is_ok());
    assert_eq!(items[1].as_ref().unwrap_err().code, code::INVALID_REQUEST);
}

#[test]
fn notifications_have_no_id() {
    let line = r#"{"jsonrpc":"2.0","method":"sim.pause"}"#;
    let Incoming::Single(r) = parse_line(line).unwrap() else {
        panic!("expected single");
    };
    assert!(r.id.is_none());
}
