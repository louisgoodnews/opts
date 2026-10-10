//! `screenshot` and `report`.

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use bevy::ecs::world::World;
use bevy::prelude::Image;
use bevy::render::view::screenshot::{CapturedScreenshots, Screenshot, ScreenshotCaptured};
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

use crate::control::handlers::input::primary_window_or_placeholder;
use crate::control::params::{opt_str, str_param};
use crate::control::paths::PathPolicy;
use crate::control::state::{ControlRegistry, Outcome, Waiter};
use crate::reporting::capture::encode_png;
use crate::reporting::format::{format_report, now_stamp, ReportMeta};
use crate::reporting::save::{reports_dir, save_report};
use crate::reporting::state::ReportContext;

/// A frame that does not arrive within this time fails the command.
const CAPTURE_WAIT: Duration = Duration::from_secs(5);

/// Registers the output methods.
pub fn register(r: &mut ControlRegistry) {
    r.register("screenshot", screenshot);
    r.register("report", report);
}

/// Requests a frame; the returned waiter yields it (or an error after [`CAPTURE_WAIT`]).
fn capture(world: &mut World) -> Result<mpsc::Receiver<Image>, RpcError> {
    let window = primary_window_or_placeholder(world);
    if !world.contains_resource::<CapturedScreenshots>() {
        return Err(RpcError::new(
            code::NOT_READY,
            "there is no renderer to take a screenshot from",
        ));
    }
    let (tx, rx) = mpsc::channel();
    world
        .spawn(Screenshot::window(window))
        .observe(move |captured: bevy::ecs::observer::On<ScreenshotCaptured>| {
            let _ = tx.send(captured.image.clone());
        });
    Ok(rx)
}

fn wait_for_png(
    rx: mpsc::Receiver<Image>,
    finish: impl Fn(Vec<u8>) -> Result<Value, RpcError> + Send + 'static,
) -> Waiter {
    let began = Instant::now();
    Box::new(move |_| match rx.try_recv() {
        Ok(img) => Some(match encode_png(&img) {
            Some(png) => finish(png),
            None => Err(RpcError::new(
                code::FAILED,
                "the frame could not be encoded as PNG",
            )),
        }),
        Err(mpsc::TryRecvError::Empty) if began.elapsed() < CAPTURE_WAIT => None,
        Err(_) => Some(Err(RpcError::new(
            code::TIMEOUT,
            "the screenshot did not arrive in time",
        ))),
    })
}

fn screenshot(world: &mut World, params: &Value) -> Outcome {
    let requested = match opt_str(params, "path") {
        Ok(p) => p,
        Err(e) => return Outcome::err(e),
    };
    let path: PathBuf = match requested {
        Some(p) => PathBuf::from(p),
        None => {
            let Some(dir) = reports_dir() else {
                return Outcome::err(RpcError::new(code::FAILED, "there is no reports directory"));
            };
            let _ = std::fs::create_dir_all(&dir);
            dir.join(format!("screenshot_{}.png", now_stamp().0))
        }
    };
    let path = match world.resource::<PathPolicy>().check_write(&path) {
        Ok(p) => p,
        Err(e) => return Outcome::err(e),
    };
    match capture(world) {
        Ok(rx) => Outcome::Wait(wait_for_png(rx, move |png| {
            std::fs::write(&path, png).map_err(|e| {
                RpcError::new(
                    code::FAILED,
                    format!("cannot write {}: {e}", path.display()),
                )
            })?;
            Ok(json!({ "path": path.display().to_string() }))
        })),
        Err(e) => Outcome::err(e),
    }
}

fn report(world: &mut World, params: &Value) -> Outcome {
    let text = match str_param(params, "text") {
        Ok(t) => t.to_string(),
        Err(e) => return Outcome::err(e),
    };
    let Some(dir) = reports_dir() else {
        return Outcome::err(RpcError::new(code::FAILED, "there is no reports directory"));
    };
    let ctx = world
        .get_resource::<ReportContext>()
        .cloned()
        .unwrap_or_default();
    match capture(world) {
        Ok(rx) => Outcome::Wait(wait_for_png(rx, move |png| {
            let (stem, timestamp) = now_stamp();
            let body = format_report(
                &ReportMeta::from_context(&ctx, timestamp, Some("control protocol")),
                &text,
            );
            let saved = save_report(&dir, &stem, &png, &body)
                .map_err(|e| RpcError::new(code::FAILED, format!("cannot save the report: {e}")))?;
            Ok(
                json!({ "png": saved.png.display().to_string(), "report": saved.report.display().to_string() }),
            )
        })),
        Err(e) => Outcome::err(e),
    }
}
