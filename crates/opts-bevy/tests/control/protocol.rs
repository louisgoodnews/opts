//! Protocol edges: malformed lines, the log tail and the exit request.

use super::*;

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
