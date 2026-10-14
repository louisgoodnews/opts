//! `screenshot` over the control socket in a headless app: parameter validation, path rules and
//! the error codes. (There is no renderer here, so no frame is ever delivered; frame handling is
//! covered by the unit tests in `capture::screenshot`.)

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use bevy::prelude::*;
use opts_bevy::capture::CaptureState;
use opts_bevy::control::{ControlAddr, ControlConfig, ControlPlugin};
use opts_cli::client::{Client, ClientConfig, ClientError};
use opts_cli::protocol::code;
use serde_json::{json, Value};

fn rpc_code(e: ClientError) -> i64 {
    match e {
        ClientError::Rpc(r) => r.code,
        other => panic!("expected an rpc error, got {other}"),
    }
}

fn app(root: &std::path::Path) -> (App, u16) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::input::InputPlugin));
    app.add_plugins(ControlPlugin::new(ControlConfig {
        port: Some(0),
        ..ControlConfig::default()
    }));
    app.insert_resource(CaptureState::new(Some(root.to_path_buf())));
    app.update();
    let port = app.world().resource::<ControlAddr>().0.port();
    (app, port)
}

fn with_client<R: Send + 'static>(
    app: &mut App,
    port: u16,
    f: impl FnOnce(Client) -> R + Send + 'static,
) -> R {
    let done = Arc::new(AtomicBool::new(false));
    let flag = done.clone();
    let handle = thread::spawn(move || {
        let r = f(Client::new(ClientConfig {
            host: "127.0.0.1".into(),
            port,
            token: None,
            timeout: Duration::from_secs(10),
        }));
        flag.store(true, Ordering::SeqCst);
        r
    });
    while !done.load(Ordering::SeqCst) {
        app.update();
        thread::sleep(Duration::from_millis(4));
    }
    handle.join().expect("client thread")
}

#[test]
fn screenshot_rejects_bad_names_paths_and_parameters() {
    let root = std::env::temp_dir().join(format!("opts_capctl_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let (mut app, port) = app(&root);
    let codes = with_client(&mut app, port, |mut c| {
        let mut call = |p: Value| rpc_code(c.call("screenshot", p).expect_err("must fail"));
        vec![
            call(json!({"name": "../../etc/passwd"})),
            call(json!({"name": "/etc/passwd"})),
            call(json!({"name": "a/../../b.png"})),
            call(json!({"path": "/etc/opts_nope.png"})),
            call(json!({"path": "/etc/opts_nope.png", "name": "x.png"})),
            call(json!({"format": "bmp"})),
            call(json!({"quality": 0})),
            // A valid name passes validation; the headless app then has no renderer.
            call(json!({"name": "run1/ok.png"})),
        ]
    });
    assert_eq!(
        codes,
        [
            code::FORBIDDEN_PATH,
            code::FORBIDDEN_PATH,
            code::FORBIDDEN_PATH,
            code::FORBIDDEN_PATH,
            code::INVALID_PARAMS,
            code::INVALID_PARAMS,
            code::INVALID_PARAMS,
            code::NOT_READY,
        ]
    );
    assert!(
        !root.join("run1").join("ok.png").exists(),
        "nothing is written without a frame"
    );
    let _ = std::fs::remove_dir_all(&root);
}
