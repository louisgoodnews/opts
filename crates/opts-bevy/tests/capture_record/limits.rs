//! Recording limit, stall, frame-rate degradation and concurrency tests over the control socket.

use super::*;

#[test]
fn a_second_recording_or_a_burst_during_a_recording_is_rejected() {
    let root = temp("busy");
    let (mut app, port) = app(&root, deliver);
    let (c1, c2, c3, stopped) = run(&mut app, port, |c| {
        let s = c
            .call("capture.record.start", json!({"duration_ms": 20_000}))
            .expect("start");
        let id = s["recording_id"].as_str().expect("id").to_string();
        let c1 = rpc_code(
            c.call("capture.record.start", json!({}))
                .expect_err("second"),
        );
        let c2 = rpc_code(
            c.call("capture.burst", json!({"count": 2, "interval_ms": 50}))
                .expect_err("burst"),
        );
        let c3 = rpc_code(
            c.call("capture.record.status", json!({"recording_id": "nope"}))
                .expect_err("unknown"),
        );
        let stopped = c
            .call("capture.record.stop", json!({"recording_id": id}))
            .expect("stop");
        (c1, c2, c3, stopped)
    });
    assert_eq!((c1, c2, c3), (code::BUSY, code::BUSY, code::NOT_FOUND));
    assert_eq!(stopped["stopped_by"], "request");
    assert!(std::fs::metadata(stopped["path"].as_str().expect("path")).is_ok());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn dropped_frames_are_reported_when_the_renderer_stalls() {
    let root = temp("stall");
    let (mut app, port) = app(&root, never_deliver);
    app.world_mut().insert_resource(CaptureLimits {
        no_delivery_after: Duration::from_millis(800),
        ..CaptureLimits::default()
    });
    let status = run(&mut app, port, |c| {
        let s = c
            .call("capture.record.start", json!({"duration_ms": 20_000}))
            .expect("start");
        wait_finished(c, s["recording_id"].as_str().expect("id"))
    });
    assert_eq!(status["stopped_by"], "window");
    assert!(status["dropped"].as_u64().unwrap_or(0) > 0, "{status}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_low_frame_rate_halves_the_rate_and_then_stops_the_recording() {
    let root = temp("slow");
    let (mut app, port) = app(&root, deliver);
    app.world_mut().insert_resource(CaptureLimits {
        degrade_below_fps: 1.0e9,
        stop_below_fps: 1.0e9,
        stop_after: Duration::from_millis(700),
        ..CaptureLimits::default()
    });
    let (saw_degraded, status) = run(&mut app, port, |c| {
        let s = c
            .call("capture.record.start", json!({"duration_ms": 20_000}))
            .expect("start");
        let id = s["recording_id"].as_str().expect("id").to_string();
        thread::sleep(Duration::from_millis(300));
        let mid = c
            .call("capture.record.status", json!({"recording_id": id}))
            .expect("status");
        (mid["degraded"] == true, wait_finished(c, &id))
    });
    assert!(
        saw_degraded,
        "the capture rate is halved while the frame rate is low"
    );
    assert_eq!(status["stopped_by"], "degraded");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_size_limit_stops_a_recording() {
    let root = temp("size");
    let (mut app, port) = app(&root, deliver);
    app.insert_resource(TestFrame(1280, 720));
    let status = run(&mut app, port, |c| {
        let s = c
            .call(
                "capture.record.start",
                json!({"duration_ms": 60_000, "format": "png_sequence", "max_size_mb": 1}),
            )
            .expect("start");
        wait_finished(c, s["recording_id"].as_str().expect("id"))
    });
    assert_eq!(status["stopped_by"], "size");
    let _ = std::fs::remove_dir_all(&root);
}
