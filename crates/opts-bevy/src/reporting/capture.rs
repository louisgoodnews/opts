//! Screenshot capture of the primary window and PNG encoding.

use std::sync::{mpsc, Mutex};
use std::time::Duration;

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use bevy::tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task};
use bevy::window::PrimaryWindow;

use super::state::{ReportModalData, ReportModalState};

/// A capture that is not delivered within this time is treated as failed.
pub const CAPTURE_TIMEOUT: Duration = Duration::from_secs(3);

/// Channel carrying the captured frame from the render callback to the main world.
#[derive(Resource)]
pub struct CaptureChannel {
    tx: mpsc::Sender<Image>,
    rx: Mutex<mpsc::Receiver<Image>>,
}

impl Default for CaptureChannel {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            tx,
            rx: Mutex::new(rx),
        }
    }
}

impl CaptureChannel {
    /// A sender that delivers a frame as if the render world had captured it (tests, tools).
    #[must_use]
    pub fn sender(&self) -> mpsc::Sender<Image> {
        self.tx.clone()
    }
}

/// PNG encoding in progress.
#[derive(Resource, Default)]
pub struct EncodeTask(Option<Task<Option<Vec<u8>>>>);

/// Encodes an RGBA/BGRA frame as PNG (alpha dropped); `None` on conversion failure.
#[must_use]
pub fn encode_png(image: &Image) -> Option<Vec<u8>> {
    let rgb = image.clone().try_into_dynamic().ok()?.to_rgb8();
    let mut out = Vec::new();
    rgb.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .ok()?;
    Some(out)
}

/// Asks the renderer for a frame of `window`; the observer forwards it to `tx`.
pub fn spawn_capture(commands: &mut Commands, window: Entity, tx: mpsc::Sender<Image>) {
    commands
        .spawn(Screenshot::window(window))
        .observe(move |captured: On<ScreenshotCaptured>| {
            let _ = tx.send(captured.image.clone());
        });
}

/// Requests the screenshot (runs on entering `Capturing`, i.e. before the modal exists).
pub fn request_screenshot(
    mut commands: Commands,
    window: Query<Entity, With<PrimaryWindow>>,
    channel: Res<CaptureChannel>,
    mut next: ResMut<NextState<ReportModalState>>,
) {
    let Ok(window) = window.single() else {
        tracing::error!("report: no primary window to capture");
        next.set(ReportModalState::Inactive);
        return;
    };
    // Drop frames left over from an earlier, abandoned capture.
    if let Ok(rx) = channel.rx.lock() {
        while rx.try_recv().is_ok() {}
    }
    spawn_capture(&mut commands, window, channel.tx.clone());
}

/// Collects the frame, encodes it off-thread, then opens the modal.
pub fn poll_capture(
    channel: Res<CaptureChannel>,
    mut task: ResMut<EncodeTask>,
    mut data: ResMut<ReportModalData>,
    mut images: ResMut<Assets<Image>>,
    mut next: ResMut<NextState<ReportModalState>>,
) {
    if data.capture_started_at.elapsed() > CAPTURE_TIMEOUT {
        tracing::error!("report: screenshot did not arrive in time; report cancelled");
        task.0 = None;
        next.set(ReportModalState::Inactive);
        return;
    }
    if task.0.is_none() {
        let frame = channel.rx.lock().ok().and_then(|rx| rx.try_recv().ok());
        let Some(frame) = frame else { return };
        data.screenshot = Some(images.add(frame.clone()));
        task.0 = Some(AsyncComputeTaskPool::get().spawn(async move { encode_png(&frame) }));
        return;
    }
    let finished = task.0.as_mut().and_then(|t| block_on(future::poll_once(t)));
    if let Some(result) = finished {
        task.0 = None;
        match result {
            Some(png) => {
                data.png = Some(png);
                next.set(ReportModalState::Open);
            }
            None => {
                tracing::error!("report: could not encode the screenshot as PNG");
                next.set(ReportModalState::Inactive);
            }
        }
    }
}
