//! The client against an in-process mock server: calls, errors, batches, reconnection, tokens.

use std::net::TcpListener;
use std::thread;
use std::time::Duration;

use opts_cli::client::ClientError;
use opts_cli::output::OutputMode;
use opts_cli::protocol::code;
use opts_cli::run::execute;
use serde_json::{json, Value};

#[path = "protocol_test/server.rs"]
mod server;
use server::{client, start, Mode};

#[test]
fn a_call_returns_the_result() {
    let server = start(0, Mode::Normal);
    let mut c = client(server.port, None);
    let r = c.call("ping", Value::Null).expect("ping");
    assert_eq!(r["pong"], true);
    assert_eq!(
        c.call("echo", json!({"a": 1})).expect("echo"),
        json!({"a": 1})
    );
}

#[test]
fn server_errors_keep_code_and_data() {
    let server = start(0, Mode::Normal);
    let mut c = client(server.port, None);
    let ClientError::Rpc(e) = c.call("boom", Value::Null).expect_err("error") else {
        panic!("expected an rpc error");
    };
    assert_eq!(e.code, code::NOT_FOUND);
    // The caller's data is kept; the wire adds the symbolic name and the retry hint.
    let data = e.data.expect("data");
    assert_eq!(data["id"], 9);
    assert_eq!(data["name"], "NOT_FOUND");
    assert_eq!(data["retryable"], false);
    let unknown = c.call("nope", Value::Null).expect_err("unknown");
    assert_eq!(unknown.exit_code(), 1);
}

#[test]
fn a_closed_port_is_a_connection_error() {
    let free = TcpListener::bind(("127.0.0.1", 0))
        .expect("bind")
        .local_addr()
        .expect("addr")
        .port();
    let mut c = client(free, None);
    let e = c.call("ping", Value::Null).expect_err("refused");
    assert!(matches!(e, ClientError::Connection(_)));
    assert_eq!(e.exit_code(), 2);
}

#[test]
fn garbage_replies_are_protocol_errors() {
    let server = start(0, Mode::Garbage);
    let mut c = client(server.port, None);
    let e = c.call("ping", Value::Null).expect_err("garbage");
    assert!(matches!(e, ClientError::Protocol(_)), "{e}");
    assert_eq!(e.exit_code(), 3);
}

#[test]
fn batch_returns_one_result_per_command() {
    let server = start(0, Mode::Normal);
    let mut c = client(server.port, None);
    let cmds = vec![
        ("ping".to_string(), Value::Null),
        ("nope".to_string(), Value::Null),
    ];
    let results = c.batch(&cmds, false).expect("batch");
    assert_eq!(results.len(), 2);
    assert!(results[0].is_ok());
    assert_eq!(
        results[1].as_ref().unwrap_err().code,
        code::METHOD_NOT_FOUND
    );
}

#[test]
fn the_client_reconnects_after_a_server_restart() {
    let first = start(0, Mode::Normal);
    let port = first.port;
    let mut c = client(port, None);
    c.call("ping", Value::Null).expect("first ping");
    drop(first);
    thread::sleep(Duration::from_millis(200));
    let _second = start(port, Mode::Normal);
    c.call("ping", Value::Null).expect("ping after restart");
}

#[test]
fn tokens_are_sent_and_required() {
    let server = start(0, Mode::RequireToken);
    let ok = client(server.port, Some("secret")).call("ping", Value::Null);
    assert!(ok.is_ok(), "{ok:?}");
    let denied = client(server.port, None)
        .call("ping", Value::Null)
        .expect_err("denied");
    assert!(
        matches!(&denied, ClientError::Rpc(e) if e.code == code::UNAUTHORIZED),
        "{denied}"
    );
    let wrong = client(server.port, Some("nope"))
        .call("ping", Value::Null)
        .expect_err("wrong");
    assert!(
        matches!(&wrong, ClientError::Rpc(e) if e.code == code::UNAUTHORIZED),
        "{wrong}"
    );
}

#[test]
fn execute_maps_outcomes_to_exit_codes() {
    let server = start(0, Mode::Normal);
    let mut c = client(server.port, None);
    let words = |w: &[&str]| w.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
    assert_eq!(execute(&mut c, OutputMode::Quiet, &words(&["ping"])), 0);
    assert_eq!(execute(&mut c, OutputMode::Quiet, &words(&["boom"])), 1);
    assert_eq!(
        execute(&mut c, OutputMode::Quiet, &words(&["sim.step"])),
        1,
        "missing parameter"
    );
    assert_eq!(
        execute(&mut c, OutputMode::Quiet, &words(&["echo", "--text", "hi"])),
        0,
        "unknown methods pass through"
    );
}
