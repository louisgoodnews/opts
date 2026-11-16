//! The encoder worker thread and the counters it shares with the main thread.

mod plan;
mod shared;
mod sidecar;

pub use plan::{Msg, WorkerPlan};
pub use shared::{Shared, FAILED, FINISHED, FINISHING, RECORDING};

use std::sync::atomic::Ordering;
use std::sync::mpsc::Receiver;
use std::time::Instant;

use super::av1::Av1Ivf;
use super::encoder::{Encoder, FrameInfo, PngSequence, RgbFrame};
use super::error::CaptureError;
use super::recording::RecFormat;
use sidecar::write_sidecar;

pub fn set_error(shared: &Shared, e: &CaptureError) {
    if let Ok(mut g) = shared.error.lock() {
        g.get_or_insert_with(|| e.to_string());
    }
}

pub fn worker(rx: &Receiver<Msg>, plan: &WorkerPlan, shared: &Shared) {
    let mut encoder: Option<Box<dyn Encoder>> = None;
    let mut size = (0, 0);
    let mut infos: Vec<FrameInfo> = Vec::new();
    let mut failed = false;
    while let Ok(Msg::Frame(image, info)) = rx.recv() {
        let step = (|| {
            let t0 = Instant::now();
            let mut frame = RgbFrame::from_image(image, plan.scale)?;
            shared
                .convert_us
                .fetch_add(t0.elapsed().as_micros() as u64, Ordering::Relaxed);
            if encoder.is_none() {
                size = (frame.width, frame.height);
                encoder = Some(make_encoder(plan, &frame)?);
            } else if (frame.width, frame.height) != size {
                frame = frame.fitted(size.0, size.1);
            }
            let t1 = Instant::now();
            let bytes = encoder.as_mut().map_or(Ok(0), |e| e.push(&frame, &info))?;
            shared
                .encode_us
                .fetch_add(t1.elapsed().as_micros() as u64, Ordering::Relaxed);
            Ok::<u64, CaptureError>(bytes)
        })();
        match step {
            Ok(bytes) => {
                shared.bytes.store(bytes, Ordering::Relaxed);
                shared.frames.fetch_add(1, Ordering::Relaxed);
                infos.push(info);
            }
            Err(e) => {
                set_error(shared, &e);
                failed = true;
                break;
            }
        }
    }
    shared.state.store(FINISHING, Ordering::Relaxed);
    if let Some(enc) = encoder {
        match enc.finish() {
            Ok(bytes) => shared.bytes.store(bytes, Ordering::Relaxed),
            Err(e) => {
                set_error(shared, &e);
                failed = true;
            }
        }
    }
    failed |= write_sidecar(plan, shared, size, &infos);
    shared
        .state
        .store(if failed { FAILED } else { FINISHED }, Ordering::Release);
}

fn make_encoder(plan: &WorkerPlan, frame: &RgbFrame) -> Result<Box<dyn Encoder>, CaptureError> {
    Ok(match plan.format {
        RecFormat::Av1 => Box::new(Av1Ivf::create(
            &plan.output,
            frame.width,
            frame.height,
            plan.fps,
            plan.quality,
        )?),
        RecFormat::PngSequence => Box::new(PngSequence::create(&plan.output)?),
    })
}
