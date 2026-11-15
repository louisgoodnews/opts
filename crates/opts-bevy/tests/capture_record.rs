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

#[path = "capture_record/av1.rs"]
mod av1;
#[path = "capture_record/burst.rs"]
mod burst;
#[path = "capture_record/disk.rs"]
mod disk;
#[path = "capture_record/limits.rs"]
mod limits;
