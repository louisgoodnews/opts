//! Requesting and encoding a frame of the primary window.

use std::sync::mpsc;

use bevy::ecs::world::World;
use bevy::prelude::*;
use bevy::render::view::screenshot::{CapturedScreenshots, Screenshot, ScreenshotCaptured};
use bevy::window::PrimaryWindow;

use super::super::error::CaptureError;
use super::options::StillFormat;

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
    sink: super::super::backend::FrameSink,
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
