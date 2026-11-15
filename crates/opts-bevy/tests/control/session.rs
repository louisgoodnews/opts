//! Session control: pause, step and the time scale, batches, token enforcement.

use super::*;

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
