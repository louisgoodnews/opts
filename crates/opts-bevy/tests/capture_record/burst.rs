//! Burst requests: the frame count and interval, and the parameter and path checks.

use super::*;

#[test]
fn a_burst_writes_n_frames_at_the_requested_interval() {
    let root = temp("burst");
    let (mut app, port) = app(&root, deliver);
    let result = run(&mut app, port, |c| {
        c.call(
            "capture.burst",
            json!({"count": 5, "interval_ms": 100, "dir": "b1"}),
        )
        .expect("burst")
    });
    let frames = result["frames"].as_array().expect("frames");
    assert_eq!(frames.len(), 5);
    assert_eq!(result["dropped"], 0);
    for (i, f) in frames.iter().enumerate() {
        let path = PathBuf::from(f["path"].as_str().expect("path"));
        assert_eq!(image::open(&path).expect("png").width(), 64, "frame {i}");
        assert!(f["sim_time"].is_number() && f["frame"].is_number());
    }
    let walls: Vec<u64> = frames
        .iter()
        .map(|f| f["wall_ms"].as_u64().expect("wall"))
        .collect();
    for w in walls.windows(2) {
        assert!((85..=250).contains(&(w[1] - w[0])), "spacing {walls:?}");
    }
    let manifest: Value = serde_json::from_slice(
        &std::fs::read(result["manifest"].as_str().expect("m")).expect("read"),
    )
    .expect("json");
    assert_eq!(manifest["frames"].as_array().map(Vec::len), Some(5));
    assert_eq!(manifest["kind"], "burst");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn burst_parameters_and_paths_are_checked() {
    let root = temp("burstp");
    let (mut app, port) = app(&root, deliver);
    let codes = run(&mut app, port, |c| {
        let mut call = |p: Value| rpc_code(c.call("capture.burst", p).expect_err("must fail"));
        vec![
            call(json!({"count": 0, "interval_ms": 100})),
            call(json!({"count": 301, "interval_ms": 100})),
            call(json!({"count": 5, "interval_ms": 5})),
            call(json!({"count": 300, "interval_ms": 1000})),
            call(json!({"count": 2, "interval_ms": 100, "dir": "../../x"})),
            call(json!({"count": 2, "interval_ms": 100, "dir": "/tmp/x"})),
        ]
    });
    assert_eq!(codes[..4], [code::INVALID_PARAMS; 4]);
    assert_eq!(codes[4..], [code::FORBIDDEN_PATH; 2]);
    let _ = std::fs::remove_dir_all(&root);
}

