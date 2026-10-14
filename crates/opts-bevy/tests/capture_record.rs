//! Bursts and recordings over the control socket in a headless app, with a synthetic frame
//! backend instead of a renderer.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use opts_bevy::capture::backend::{CaptureBackend, FrameSink};
use opts_bevy::capture::error::CaptureError;
use opts_bevy::capture::ivf;
use opts_bevy::capture::jobs::CaptureLimits;
use opts_bevy::capture::CaptureState;
use opts_bevy::control::{ControlAddr, ControlConfig, ControlPlugin};
use opts_cli::client::{Client, ClientConfig, ClientError};
use opts_cli::protocol::code;
use serde_json::{json, Value};

/// Size of the synthetic frames, per app (tests run in parallel).
#[derive(Resource, Clone, Copy)]
struct TestFrame(u32, u32);

static COUNTER: AtomicU32 = AtomicU32::new(0);

fn synthetic_image(w: u32, h: u32) -> Image {
    let t = COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut data = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let n = (x * 7 + y * 13 + t * 11) ^ (x * y);
            data.extend_from_slice(&[
                (n % 256) as u8,
                ((n >> 3) % 256) as u8,
                ((x + t) % 256) as u8,
                255,
            ]);
        }
    }
    Image::new(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

fn deliver(world: &mut World, sink: FrameSink) -> Result<(), CaptureError> {
    let TestFrame(w, h) = *world.resource::<TestFrame>();
    sink(synthetic_image(w, h));
    Ok(())
}

fn never_deliver(_: &mut World, sink: FrameSink) -> Result<(), CaptureError> {
    drop(sink);
    Ok(())
}

fn ready(_: &mut World) -> Option<CaptureError> {
    None
}

fn temp(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("opts_caprec_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    p
}

fn app(root: &Path, request: fn(&mut World, FrameSink) -> Result<(), CaptureError>) -> (App, u16) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::input::InputPlugin));
    app.add_plugins(ControlPlugin::new(ControlConfig {
        port: Some(0),
        ..ControlConfig::default()
    }));
    app.insert_resource(CaptureState::new(Some(root.to_path_buf())));
    app.insert_resource(CaptureBackend { request, ready });
    app.insert_resource(TestFrame(64, 48));
    app.update();
    let port = app.world().resource::<ControlAddr>().0.port();
    (app, port)
}

fn run<R: Send + 'static>(
    app: &mut App,
    port: u16,
    f: impl FnOnce(&mut Client) -> R + Send + 'static,
) -> R {
    let done = Arc::new(AtomicBool::new(false));
    let flag = done.clone();
    let handle = thread::spawn(move || {
        let mut c = Client::new(ClientConfig {
            host: "127.0.0.1".into(),
            port,
            token: None,
            timeout: Duration::from_secs(30),
        });
        let r = f(&mut c);
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

/// Polls `status` until the recording has finished; returns the final status.
fn wait_finished(c: &mut Client, id: &str) -> Value {
    for _ in 0..600 {
        let s = c
            .call("capture.record.status", json!({"recording_id": id}))
            .expect("status");
        if matches!(s["state"].as_str(), Some("finished" | "failed")) {
            return s;
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("the recording did not finish");
}

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

#[test]
fn an_av1_recording_is_a_well_formed_ivf_that_stops_at_the_duration_limit() {
    let root = temp("av1");
    let (mut app, port) = app(&root, deliver);
    let (start, status) = run(&mut app, port, |c| {
        let start = c
            .call(
                "capture.record.start",
                json!({"name": "clip", "fps": 30, "duration_ms": 600, "scale": 1.0}),
            )
            .expect("start");
        let id = start["recording_id"].as_str().expect("id").to_string();
        let status = wait_finished(c, &id);
        (start, status)
    });
    assert_eq!(status["state"], "finished");
    assert_eq!(status["stopped_by"], "duration");
    let took = status["duration_ms"].as_u64().expect("duration");
    assert!(
        (600..=1600).contains(&took),
        "stops within 1 s of the limit, took {took} ms"
    );
    let path = PathBuf::from(start["path"].as_str().expect("path"));
    let bytes = std::fs::read(&path).expect("ivf");
    let info = ivf::inspect(&bytes).expect("well-formed ivf");
    let frames = status["frame_count"].as_u64().expect("frames") as u32;
    assert!(frames >= 8, "got {frames} frames");
    assert_eq!((info.fourcc, info.width, info.height), (*b"AV01", 64, 48));
    assert_eq!((info.header_frames, info.counted_frames), (frames, frames));
    let side: Value = serde_json::from_slice(
        &std::fs::read(format!("{}.json", path.display())).expect("sidecar"),
    )
    .expect("json");
    assert_eq!(
        side["frames"].as_array().map(Vec::len),
        Some(frames as usize)
    );
    assert!(side["frames"][0]["sim_time"].is_number());
    if let Ok(out) = std::process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-count_packets",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=codec_name,nb_read_packets",
            "-of",
            "csv=p=0",
        ])
        .arg(&path)
        .output()
    {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
            assert_eq!(
                text,
                format!("av1,{frames}"),
                "ffprobe agrees with the header"
            );
        }
    }
    // The documented remux path (`ffmpeg -c copy`) works on our own writer's output.
    for ext in ["mp4", "mkv", "webm"] {
        let out_path = root.join(format!("remux.{ext}"));
        let remux = std::process::Command::new("ffmpeg")
            .args(["-v", "error", "-y", "-i"])
            .arg(&path)
            .args(["-c", "copy"])
            .arg(&out_path)
            .status();
        if let Ok(status) = remux {
            assert!(status.success(), "ffmpeg -c copy to {ext}");
            assert!(std::fs::metadata(&out_path)
                .map(|m| m.len() > 0)
                .unwrap_or(false));
        }
    }
    let _ = std::fs::remove_dir_all(&root);
}

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
