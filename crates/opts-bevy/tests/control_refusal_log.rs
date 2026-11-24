//! ISS-002: a control request refused for missing authentication is logged at DEBUG under
//! `opts_bevy::control`, with the method, the peer address and the error code, and never the token.
//!
//! Its own test binary: the global tracing subscriber set here must not clash with the one that
//! `tests/control/protocol.rs` installs.

use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bevy::prelude::*;
use opts_bevy::control::{ControlAddr, ControlConfig, ControlPlugin};
use opts_cli::client::{Client, ClientConfig, ClientError};
use opts_cli::protocol::code;
use serde_json::Value;

const TEST_TOKEN: &str = "test-token-with-at-least-32-characters";

/// Writes every log line into a shared buffer the test reads back.
#[derive(Clone)]
struct Capture(Arc<Mutex<Vec<u8>>>);

impl Write for Capture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().expect("capture").extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn app(token: Option<&str>) -> (App, u16) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(ControlPlugin::new(ControlConfig {
        port: Some(0),
        token: token.map(str::to_string),
        ..ControlConfig::default()
    }));
    app.insert_resource(opts_bevy::capture::CaptureState::new(Some(
        std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("opts_refusal_log_{}", std::process::id())),
    )));
    app.update();
    let port = app.world().resource::<ControlAddr>().0.port();
    (app, port)
}

#[test]
fn an_unauthenticated_ping_is_logged_without_the_token() {
    let buf = Arc::new(Mutex::new(Vec::new()));
    let sink = Capture(buf.clone());
    let subscriber = tracing_subscriber::fmt()
        .with_writer(move || sink.clone())
        .with_max_level(tracing::Level::DEBUG)
        .with_ansi(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("subscriber");

    let (mut app, port) = app(Some(TEST_TOKEN));
    // The client sends no token, so the ping is refused before dispatch.
    let refused = std::thread::spawn(move || {
        let mut c = Client::new(ClientConfig {
            host: "127.0.0.1".into(),
            port,
            token: None,
            timeout: Duration::from_secs(10),
        });
        c.call("ping", Value::Null)
    });
    while !refused.is_finished() {
        app.update();
        std::thread::sleep(Duration::from_millis(4));
    }
    match refused.join().expect("client thread") {
        Err(ClientError::Rpc(r)) => assert_eq!(r.code, code::UNAUTHORIZED),
        other => panic!("expected error 1003, got {other:?}"),
    }

    let log = String::from_utf8(buf.lock().expect("capture").clone()).expect("utf-8 log");
    let line = log
        .lines()
        .find(|l| l.contains("request refused: not authenticated"))
        .unwrap_or_else(|| panic!("no refusal line in the log:\n{log}"));
    assert!(line.contains("DEBUG"), "{line}");
    assert!(line.contains("opts_bevy::control"), "{line}");
    assert!(line.contains("method=ping"), "{line}");
    assert!(line.contains("peer=127.0.0.1"), "{line}");
    assert!(line.contains("code=1003"), "{line}");
    assert!(
        !log.contains(TEST_TOKEN),
        "the token reached the log:\n{log}"
    );
}
