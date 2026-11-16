//! The in-process mock server of the protocol tests: it answers the protocol according to a `Mode`.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use opts_cli::client::{Client, ClientConfig};
use opts_cli::protocol::{code, parse_line, Incoming, Request, Response, RpcError};
use serde_json::{json, Value};

/// A tiny server speaking the protocol; behaviour is selected by `Mode`.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Mode {
    Normal,
    RequireToken,
    Garbage,
}

pub(super) struct Mock {
    pub(super) port: u16,
    pub(super) stop: Arc<AtomicBool>,
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

pub(super) fn start(port: u16, mode: Mode) -> Mock {
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

pub(super) fn client(port: u16, token: Option<&str>) -> Client {
    Client::new(ClientConfig {
        host: "127.0.0.1".into(),
        port,
        token: token.map(str::to_string),
        timeout: Duration::from_secs(2),
    })
}
