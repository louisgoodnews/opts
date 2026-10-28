//! One-shot capture of the primary window: request a frame, encode it off the main thread,
//! write the file and its metadata sidecar.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use bevy::ecs::world::World;
use bevy::prelude::*;
use bevy::render::view::screenshot::{CapturedScreenshots, Screenshot, ScreenshotCaptured};
use bevy::tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task};
use bevy::window::PrimaryWindow;
use opts_cli::protocol::RpcError;
use serde_json::{json, Map, Value};

use crate::control::params::{opt_bool, opt_str, opt_u64};
use crate::control::state::{Outcome, Waiter};

use super::error::CaptureError;
use super::metadata::{self, Subject};
use super::storage::{self, CaptureState};

/// A frame that does not arrive within this time fails the command.
pub const FRAME_WAIT: Duration = Duration::from_secs(5);

/// Default JPEG quality.
pub const DEFAULT_JPEG_QUALITY: u8 = 90;

/// Still image formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StillFormat {
    /// Lossless, the default.
    Png,
    /// Lossy, for size.
    Jpeg,
}

impl StillFormat {
    /// Parses the `format` parameter.
    ///
    /// # Errors
    /// `InvalidParams` naming the valid values.
    pub fn parse(s: &str) -> Result<Self, CaptureError> {
        match s {
            "png" => Ok(Self::Png),
            "jpeg" | "jpg" => Ok(Self::Jpeg),
            other => Err(CaptureError::InvalidParams(format!(
                "unknown format `{other}`; valid: png, jpeg"
            ))),
        }
    }

    /// Format name as reported in results and metadata.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpeg",
        }
    }

    /// File extension.
    #[must_use]
    pub const fn ext(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
        }
    }
}

/// Where a still goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// A generated name in the captures directory.
    Generated,
    /// A relative name below the captures directory (`name`).
    Name(String),
    /// An explicit path that must lie inside the captures directory (`path`).
    Path(PathBuf),
}

/// Parsed parameters of `screenshot`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// Destination.
    pub target: Target,
    /// Still format.
    pub format: StillFormat,
    /// JPEG quality (1..=100).
    pub quality: u8,
    /// Whether to write the sidecar and return the metadata.
    pub include_metadata: bool,
}

/// Reads `path`, `name`, `format`, `quality` and `include_metadata`.
///
/// # Errors
/// `InvalidParams` for mistyped or conflicting parameters.
pub fn parse_options(params: &Value) -> Result<Options, RpcError> {
    let path = opt_str(params, "path")?;
    let name = opt_str(params, "name")?;
    let target = match (path, name) {
        (Some(_), Some(_)) => {
            return Err(RpcError::invalid_params(
                "`path` and `name` are mutually exclusive; prefer `name`",
            ))
        }
        (Some(p), None) => Target::Path(PathBuf::from(p)),
        (None, Some(n)) => Target::Name(n.to_string()),
        (None, None) => Target::Generated,
    };
    let format = match opt_str(params, "format")? {
        Some(f) => StillFormat::parse(f)?,
        None => StillFormat::Png,
    };
    let quality = match opt_u64(params, "quality")? {
        Some(q @ 1..=100) => q as u8,
        Some(_) => return Err(RpcError::invalid_params("`quality` must be 1..=100")),
        None => DEFAULT_JPEG_QUALITY,
    };
    Ok(Options {
        target,
        format,
        quality,
        include_metadata: opt_bool(params, "include_metadata")?.unwrap_or(true),
    })
}

/// Why no frame can be requested right now (no renderer, nothing visible), if so.
pub fn not_ready(world: &mut World) -> Option<CaptureError> {
    if !world.contains_resource::<CapturedScreenshots>() {
        return Some(CaptureError::NotReady(
            "there is no renderer to take a screenshot from".into(),
        ));
    }
    let mut windows = world.query_filtered::<&Window, With<PrimaryWindow>>();
    if let Some(w) = windows.iter(world).next() {
        if w.resolution.physical_width() == 0 || w.resolution.physical_height() == 0 {
            return Some(CaptureError::NotReady(
                "the window is minimized, so nothing is rendered; restore it and try again".into(),
            ));
        }
    }
    None
}

/// Requests a frame of the primary window; the receiver yields it once the renderer delivers it.
///
/// # Errors
/// `NotReady` without a renderer or while the window is minimized.
pub fn request_frame(world: &mut World) -> Result<mpsc::Receiver<Image>, CaptureError> {
    let (tx, rx) = mpsc::channel();
    request_with(
        world,
        Box::new(move |image| {
            let _ = tx.send(image);
        }),
    )?;
    Ok(rx)
}

/// Requests a frame of the primary window and hands it to `sink` when the renderer delivers it.
///
/// # Errors
/// `NotReady` without a renderer or while the window is minimized.
pub fn request_with(
    world: &mut World,
    sink: super::backend::FrameSink,
) -> Result<(), CaptureError> {
    if let Some(e) = not_ready(world) {
        return Err(e);
    }
    let mut windows = world.query_filtered::<Entity, With<PrimaryWindow>>();
    let window = windows.iter(world).next().unwrap_or(Entity::PLACEHOLDER);
    let sink = std::sync::Mutex::new(Some(sink));
    world.spawn(Screenshot::window(window)).observe(
        move |captured: bevy::ecs::observer::On<ScreenshotCaptured>| {
            let taken = sink.lock().ok().and_then(|mut g| g.take());
            if let Some(sink) = taken {
                sink(captured.image.clone());
            }
        },
    );
    Ok(())
}

/// Encodes a frame (alpha dropped) as PNG or JPEG.
///
/// # Errors
/// `Failed` when the pixel format cannot be converted or encoding fails.
pub fn encode(image: Image, format: StillFormat, quality: u8) -> Result<Vec<u8>, CaptureError> {
    let rgb = image
        .try_into_dynamic()
        .map_err(|e| CaptureError::Failed(format!("the frame cannot be converted: {e}")))?
        .to_rgb8();
    let mut out = Vec::new();
    match format {
        StillFormat::Png => rgb
            .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
            .map_err(|e| CaptureError::Failed(format!("PNG encoding failed: {e}")))?,
        StillFormat::Jpeg => {
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality)
                .encode_image(&rgb)
                .map_err(|e| CaptureError::Failed(format!("JPEG encoding failed: {e}")))?;
        }
    }
    Ok(out)
}

/// The sidecar path of a capture file: its name with `.json` appended.
#[must_use]
pub fn sidecar_path(file: &Path) -> PathBuf {
    let mut s = file.as_os_str().to_owned();
    s.push(".json");
    PathBuf::from(s)
}

/// Encodes `image`, writes it to `path` and (with `meta`) the sidecar; returns the reply.
///
/// Runs on a worker thread. The sidecar is written after the image, so its presence means the
/// image is complete.
///
/// # Errors
/// `Failed` on encoding or I/O errors.
pub fn finish(
    image: Image,
    path: &Path,
    options: &Options,
    meta: Option<Map<String, Value>>,
) -> Result<Value, CaptureError> {
    let (width, height) = (image.width(), image.height());
    let bytes = encode(image, options.format, options.quality)?;
    storage::write_atomic(path, &bytes)?;
    let mut reply = Map::new();
    reply.insert("path".into(), json!(path.display().to_string()));
    reply.insert("width".into(), json!(width));
    reply.insert("height".into(), json!(height));
    reply.insert("format".into(), json!(options.format.name()));
    reply.insert("bytes".into(), json!(bytes.len()));
    if let Some(mut meta) = meta {
        meta.insert("width".into(), json!(width));
        meta.insert("height".into(), json!(height));
        let side = sidecar_path(path);
        let text = serde_json::to_vec_pretty(&meta)
            .map_err(|e| CaptureError::Failed(format!("metadata serialisation failed: {e}")))?;
        storage::write_atomic(&side, &text)?;
        for key in ["sim_time", "frame"] {
            if let Some(v) = meta.get(key) {
                reply.insert(key.into(), v.clone());
            }
        }
        reply.insert("metadata_path".into(), json!(side.display().to_string()));
        reply.insert("metadata".into(), Value::Object(meta));
    }
    Ok(Value::Object(reply))
}

fn resolve_target(world: &mut World, options: &Options) -> Result<PathBuf, CaptureError> {
    let mut state = world.get_resource_or_insert_with(CaptureState::default);
    let root = state.ensure_root()?;
    match &options.target {
        Target::Generated => {
            let stem = state.next_stem();
            storage::resolve_name(&root, &format!("{stem}.{}", options.format.ext()))
        }
        Target::Name(n) => storage::resolve_name(&root, n),
        Target::Path(p) => storage::resolve_explicit(&root, p),
    }
}

enum Stage {
    Waiting(mpsc::Receiver<Image>, Instant),
    Encoding(Task<Result<Value, CaptureError>>),
}

/// The `screenshot` command.
pub fn handle(world: &mut World, params: &Value) -> Outcome {
    let options = match parse_options(params) {
        Ok(o) => o,
        Err(e) => return Outcome::err(e),
    };
    let path = match resolve_target(world, &options) {
        Ok(p) => p,
        Err(e) => return Outcome::err(e.into()),
    };
    let rx = match request_frame(world) {
        Ok(rx) => rx,
        Err(e) => return Outcome::err(e.into()),
    };
    let meta = options.include_metadata.then(|| {
        let file = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        metadata::collect(
            world,
            Subject {
                kind: "screenshot",
                file: &file,
                width: 0,
                height: 0,
                format: options.format.name(),
            },
        )
    });
    Outcome::Wait(waiter(rx, path, options, meta))
}

fn waiter(
    rx: mpsc::Receiver<Image>,
    path: PathBuf,
    options: Options,
    mut meta: Option<Map<String, Value>>,
) -> Waiter {
    let mut stage = Some(Stage::Waiting(rx, Instant::now()));
    Box::new(move |_| match stage.take()? {
        Stage::Waiting(rx, began) => match rx.try_recv() {
            Ok(image) => {
                let (path, options, meta) = (path.clone(), options.clone(), meta.take());
                let pool = AsyncComputeTaskPool::get_or_init(bevy::tasks::TaskPool::default);
                stage = Some(Stage::Encoding(
                    pool.spawn(async move { finish(image, &path, &options, meta) }),
                ));
                None
            }
            Err(mpsc::TryRecvError::Empty) if began.elapsed() < FRAME_WAIT => {
                stage = Some(Stage::Waiting(rx, began));
                None
            }
            Err(_) => Some(Err(CaptureError::Timeout(
                "the frame did not arrive in time (is the window occluded?)".into(),
            )
            .into())),
        },
        Stage::Encoding(mut task) => match block_on(future::poll_once(&mut task)) {
            Some(result) => Some(result.map_err(RpcError::from)),
            None => {
                stage = Some(Stage::Encoding(task));
                None
            }
        },
    })
}

#[cfg(test)]
mod tests;
