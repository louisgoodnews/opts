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
    // Captures go to a throw-away directory, never the user's.
    app.insert_resource(opts_bevy::capture::CaptureState::new(Some(
        std::env::temp_dir().join(format!("opts_control_test_{}", std::process::id())),
    )));
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

#[path = "control/keys.rs"]
mod keys;
#[path = "control/protocol.rs"]
mod protocol;
#[path = "control/session.rs"]
mod session;

