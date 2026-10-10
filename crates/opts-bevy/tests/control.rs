//! The control server inside a headless Bevy app: a client thread talks to the real socket while
//! the test thread pumps `app.update()`.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use bevy::input::ButtonInput;
use bevy::prelude::*;
use bevy::time::Virtual;
use opts_bevy::control::{ControlAddr, ControlConfig, ControlPlugin};
use opts_cli::client::{Client, ClientConfig, ClientError};
use opts_cli::protocol::code;
use serde_json::{json, Value};

#[derive(Resource, Default, Clone)]
struct KeyLog(Arc<Mutex<Vec<bool>>>);

fn record_w(keys: Res<ButtonInput<KeyCode>>, log: Res<KeyLog>) {
    log.0.lock().expect("log").push(keys.pressed(KeyCode::KeyW));
}

fn app(token: Option<&str>) -> (App, u16) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::input::InputPlugin));
    app.init_resource::<KeyLog>().add_systems(Update, record_w);
    app.add_plugins(ControlPlugin::new(ControlConfig {
        port: Some(0),
        token: token.map(str::to_string),
        ..ControlConfig::default()
    }));
    app.update();
    let port = app.world().resource::<ControlAddr>().0.port();
    (app, port)
}

fn client(port: u16, token: Option<&str>) -> Client {
    Client::new(ClientConfig {
        host: "127.0.0.1".into(),
        port,
        token: token.map(str::to_string),
        timeout: Duration::from_secs(10),
    })
}

/// Runs `f` on a client thread while pumping the app until it returns.
fn with_client<R: Send + 'static>(app: &mut App, f: impl FnOnce() -> R + Send + 'static) -> R {
    let done = Arc::new(AtomicBool::new(false));
    let flag = done.clone();
    let handle = thread::spawn(move || {
        let r = f();
        flag.store(true, Ordering::SeqCst);
        r
    });
    while !done.load(Ordering::SeqCst) {
        app.update();
        thread::sleep(Duration::from_millis(4));
    }
    handle.join().expect("client thread")
}

fn rpc_code(e: ClientError) -> i64 {
    match e {
        ClientError::Rpc(r) => r.code,
        other => panic!("expected an rpc error, got {other}"),
    }
}

#[test]
fn meta_commands_answer() {
    let (mut app, port) = app(None);
    let (ping, version, caps, help) = with_client(&mut app, move || {
        let mut c = client(port, None);
        (
            c.call("ping", Value::Null).expect("ping"),
            c.call("version", Value::Null).expect("version"),
            c.call("capabilities", Value::Null).expect("capabilities"),
            c.call("help", json!({"command": "input.key"}))
                .expect("help"),
        )
    });
    assert_eq!(ping["pong"], true);
    assert_eq!(
        version["protocol_version"],
        opts_cli::protocol::PROTOCOL_VERSION
    );
    assert_eq!(caps["groups"]["meta"][0]["method"], "ping");
    assert_eq!(help["implemented"], true);
}

#[test]
fn errors_use_the_documented_codes() {
    let (mut app, port) = app(None);
    let codes = with_client(&mut app, move || {
        let mut c = client(port, None);
        vec![
            rpc_code(c.call("no.such.method", Value::Null).expect_err("unknown")),
            rpc_code(
                c.call("entity.list", Value::Null)
                    .expect_err("unimplemented"),
            ),
            rpc_code(
                c.call("input.key", json!({"key": "Nonsense"}))
                    .expect_err("bad key"),
            ),
            rpc_code(c.call("input.key", Value::Null).expect_err("missing")),
            rpc_code(
                c.call("screenshot", json!({"path": "/etc/opts_nope.png"}))
                    .expect_err("forbidden"),
            ),
        ]
    });
    assert_eq!(
        codes,
        [
            code::METHOD_NOT_FOUND,
            code::UNSUPPORTED,
            code::INVALID_PARAMS,
            code::INVALID_PARAMS,
            code::FORBIDDEN_PATH
        ]
    );
}

#[test]
fn injected_keys_press_then_release() {
    let (mut app, port) = app(None);
    with_client(&mut app, move || {
        client(port, None)
            .call("input.key", json!({"key": "W", "duration_ms": 120}))
            .expect("key")
    });
    for _ in 0..60 {
        app.update();
        thread::sleep(Duration::from_millis(5));
    }
    let log = app
        .world()
        .resource::<KeyLog>()
        .0
        .lock()
        .expect("log")
        .clone();
    let first = log.iter().position(|p| *p).expect("W was pressed");
    let last = log.iter().rposition(|p| *p).expect("W stayed down");
    assert!(last >= first);
    assert!(!log.last().copied().unwrap_or(true), "W is released again");
}

#[test]
fn pause_step_and_time_scale() {
    let (mut app, port) = app(None);
    let (paused, stepped, scale) = with_client(&mut app, move || {
        let mut c = client(port, None);
        c.call("sim.pause", Value::Null).expect("pause");
        let paused = c.call("sim.get", Value::Null).expect("get")["paused"].clone();
        let stepped = c.call("sim.step", json!({"frames": 3})).expect("step");
        let scale = c
            .call("sim.time_scale", json!({"factor": 2.0}))
            .expect("scale");
        (paused, stepped, scale)
    });
    assert_eq!(paused, true);
    assert_eq!(stepped["stepped"], 3);
    assert_eq!(scale["time_scale"], 2.0);
    assert!(
        app.world().resource::<Time<Virtual>>().is_paused(),
        "paused again after stepping"
    );
}

#[test]
fn batch_runs_in_order_and_reports_each_result() {
    let (mut app, port) = app(None);
    let results = with_client(&mut app, move || {
        let cmds = vec![
            ("sim.pause".to_string(), Value::Null),
            ("sim.step".to_string(), json!({"frames": 2})),
            ("nope".to_string(), Value::Null),
            ("sim.get".to_string(), Value::Null),
        ];
        client(port, None).batch(&cmds, false).expect("batch")
    });
    assert_eq!(results.len(), 4);
    assert!(results[1].is_ok());
    assert_eq!(
        results[2].as_ref().unwrap_err().code,
        code::METHOD_NOT_FOUND
    );
    assert_eq!(results[3].as_ref().expect("get")["paused"], true);
}

#[test]
fn stop_on_error_ends_the_batch_early() {
    let (mut app, port) = app(None);
    let results = with_client(&mut app, move || {
        let cmds = vec![
            ("nope".to_string(), Value::Null),
            ("ping".to_string(), Value::Null),
        ];
        client(port, None).batch(&cmds, true).expect("batch")
    });
    assert_eq!(results.len(), 1);
}

#[test]
fn tokens_are_enforced() {
    let (mut app, port) = app(Some("s3cret"));
    let (denied, allowed) = with_client(&mut app, move || {
        let denied = rpc_code(
            client(port, None)
                .call("ping", Value::Null)
                .expect_err("denied"),
        );
        let allowed = client(port, Some("s3cret"))
            .call("ping", Value::Null)
            .is_ok();
        (denied, allowed)
    });
    assert_eq!(denied, code::UNAUTHORIZED);
    assert!(allowed);
}

#[test]
fn malformed_lines_get_protocol_errors() {
    let (mut app, port) = app(None);
    let reply = with_client(&mut app, move || {
        let mut s = TcpStream::connect(("127.0.0.1", port)).expect("connect");
        writeln!(s, "{{nope").expect("write");
        let mut line = String::new();
        BufReader::new(s).read_line(&mut line).expect("read");
        serde_json::from_str::<Value>(&line).expect("json")
    });
    assert_eq!(reply["error"]["code"], code::PARSE_ERROR);
}

#[test]
fn log_tail_returns_recent_events() {
    use tracing_subscriber::layer::SubscriberExt;
    let _ = tracing::subscriber::set_global_default(
        tracing_subscriber::registry().with(opts_logging::layer::ring::RingLayer),
    );
    let (mut app, port) = app(None);
    tracing::info!(target: "control_test", "hello from the test");
    let lines = with_client(&mut app, move || {
        client(port, None)
            .call(
                "log.filter",
                json!({"contains": "hello from the test", "n": 5}),
            )
            .expect("log")
    });
    assert!(
        lines["lines"].as_array().is_some_and(|l| !l.is_empty()),
        "{lines}"
    );
}

#[derive(Resource, Default)]
struct ExitSeen(bool);

fn note_exit(mut events: MessageReader<bevy::app::AppExit>, mut seen: ResMut<ExitSeen>) {
    seen.0 |= events.read().count() > 0;
}

#[test]
fn shutdown_requests_app_exit() {
    let (mut app, port) = app(None);
    app.init_resource::<ExitSeen>()
        .add_systems(Update, note_exit);
    with_client(&mut app, move || {
        client(port, None)
            .call("shutdown", Value::Null)
            .expect("shutdown")
    });
    app.update();
    assert!(app.world().resource::<ExitSeen>().0, "AppExit was sent");
}
