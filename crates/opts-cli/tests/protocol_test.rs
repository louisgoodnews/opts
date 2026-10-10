//! The client against an in-process mock server: calls, errors, batches, reconnection, tokens.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use opts_cli::client::{Client, ClientConfig, ClientError};
use opts_cli::output::OutputMode;
use opts_cli::protocol::{code, parse_line, Incoming, Request, Response, RpcError};
use opts_cli::run::execute;
use serde_json::{json, Value};

/// A tiny server speaking the protocol; behaviour is selected by `Mode`.
#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Normal,
    RequireToken,
    Garbage,
}

struct Mock {
    port: u16,
    stop: Arc<AtomicBool>,
}

impl Drop for Mock {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Wake the accept loop so the listener is released.
        let _ = TcpStream::connect(("127.0.0.1", self.port));
    }
}

fn answer(req: &Request, authed: &mut bool, mode: Mode) -> Response {
    let id = req.id.clone().unwrap_or(Value::Null);
    if mode == Mode::RequireToken && !*authed && req.method != "auth" {
        return Response::err(id, RpcError::new(code::UNAUTHORIZED, "authenticate first"));
    }
    match req.method.as_str() {
        "auth" => {
            if req.params["token"] == "secret" {
                *authed = true;
                Response::ok(id, json!({"ok": true}))
            } else {
                Response::err(id, RpcError::new(code::UNAUTHORIZED, "bad token"))
            }
        }
        "ping" => Response::ok(id, json!({"pong": true, "uptime_secs": 1})),
        "echo" => Response::ok(id, req.params.clone()),
        "boom" => Response::err(
            id,
            RpcError::new(code::NOT_FOUND, "nothing here").with_data(json!({"id": 9})),
        ),
        "batch" => {
            let results: Vec<Value> = req.params["commands"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|c| match c["method"].as_str() {
                    Some("ping") => json!({"result": {"pong": true}}),
                    _ => json!({"error": {"code": code::METHOD_NOT_FOUND, "message": "unknown"}}),
                })
                .collect();
            Response::ok(id, json!({ "results": results }))
        }
        _ => Response::err(id, RpcError::new(code::METHOD_NOT_FOUND, "unknown method")),
    }
}

fn serve(stream: TcpStream, mode: Mode) {
    let mut writer = stream.try_clone().expect("clone");
    let mut authed = false;
    for line in BufReader::new(stream).lines().map_while(Result::ok) {
        if mode == Mode::Garbage {
            let _ = writeln!(writer, "this is not json");
            continue;
        }
        let reply = match parse_line(&line) {
            Ok(Incoming::Single(req)) => serde_json::to_string(&answer(&req, &mut authed, mode)),
            Ok(Incoming::Batch(items)) => {
                let all: Vec<Response> = items
                    .iter()
                    .map(|r| match r {
                        Ok(req) => answer(req, &mut authed, mode),
                        Err(e) => Response::err(Value::Null, e.clone()),
                    })
                    .collect();
                serde_json::to_string(&all)
            }
            Err(e) => serde_json::to_string(&Response::err(Value::Null, e)),
        };
        if writeln!(writer, "{}", reply.expect("serialisable")).is_err() {
            break;
        }
    }
}

fn start(port: u16, mode: Mode) -> Mock {
    let listener = TcpListener::bind(("127.0.0.1", port)).expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    thread::spawn(move || {
        for stream in listener.incoming() {
            if flag.load(Ordering::SeqCst) {
                break;
            }
            if let Ok(s) = stream {
                let flag = flag.clone();
                thread::spawn(move || {
                    if !flag.load(Ordering::SeqCst) {
                        serve(s, mode);
                    }
                });
            }
        }
    });
    Mock { port, stop }
}

fn client(port: u16, token: Option<&str>) -> Client {
    Client::new(ClientConfig {
        host: "127.0.0.1".into(),
        port,
        token: token.map(str::to_string),
        timeout: Duration::from_secs(2),
    })
}

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
    assert_eq!(e.data, Some(json!({"id": 9})));
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
