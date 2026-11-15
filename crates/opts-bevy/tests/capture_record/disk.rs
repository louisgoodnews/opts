//! The disk threshold: a recording refuses to start, or stops, on a nearly full disk.

use super::*;

#[test]
fn a_recording_refuses_to_start_on_a_nearly_full_disk() {
    let root = temp("disk");
    let (mut app, port) = app(&root, deliver);
    app.world_mut().resource_mut::<CaptureState>().free_space = |_| Some(100 * 1024 * 1024);
    let (code_full, code_ok) = run(&mut app, port, |c| {
        let full = rpc_code(
            c.call("capture.record.start", json!({"duration_ms": 200}))
                .expect_err("full"),
        );
        (full, 0)
    });
    assert_eq!((code_full, code_ok), (code::FAILED, 0));
    app.world_mut().resource_mut::<CaptureState>().free_space = |_| Some(50 * 1024 * 1024 * 1024);
    let ok = run(&mut app, port, |c| {
        let s = c
            .call("capture.record.start", json!({"duration_ms": 200}))
            .expect("start");
        let id = s["recording_id"].as_str().expect("id").to_string();
        wait_finished(c, &id)["state"].as_str().map(str::to_string)
    });
    assert_eq!(
        ok.as_deref(),
        Some("finished"),
        "a refused start leaves no job behind"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_disk_threshold_stops_a_running_recording() {
    let root = temp("diskstop");
    let (mut app, port) = app(&root, deliver);
    app.world_mut().resource_mut::<CaptureState>().free_space = |_| Some(50 * 1024 * 1024 * 1024);
    let status = run(&mut app, port, |c| {
        let s = c
            .call(
                "capture.record.start",
                json!({"duration_ms": 20_000, "format": "png_sequence"}),
            )
            .expect("start");
        thread::sleep(Duration::from_millis(300));
        s["recording_id"].as_str().expect("id").to_string()
    });
    app.world_mut().resource_mut::<CaptureState>().free_space = |_| Some(10 * 1024 * 1024);
    let status = run(&mut app, port, move |c| wait_finished(c, &status));
    assert_eq!(status["stopped_by"], "disk");
    assert!(
        status["frame_count"].as_u64().unwrap_or(0) > 0,
        "the partial recording is kept"
    );
    let _ = std::fs::remove_dir_all(&root);
}

