//! Injected keys: a key is pressed and then released through the control protocol.

use super::*;

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
