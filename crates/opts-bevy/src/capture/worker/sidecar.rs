//! The sidecar file of a recording: its metadata, the frame list and any error.

use std::sync::atomic::Ordering;

use serde_json::json;

use super::super::encoder::FrameInfo;
use super::super::error::CaptureError;
use super::super::storage;
use super::{set_error, Shared, WorkerPlan};

/// Writes the sidecar file of the recording; returns whether that failed.
pub(super) fn write_sidecar(
    plan: &WorkerPlan,
    shared: &Shared,
    size: (u32, u32),
    infos: &[FrameInfo],
) -> bool {
    let mut failed = false;
    let mut meta = plan.base.clone();
    meta.insert("format".into(), json!(plan.format.name()));
    meta.insert("fps".into(), json!(plan.fps));
    meta.insert("width".into(), json!(size.0));
    meta.insert("height".into(), json!(size.1));
    meta.insert("frame_count".into(), json!(infos.len()));
    meta.insert(
        "dropped".into(),
        json!(shared.dropped.load(Ordering::Relaxed)),
    );
    meta.insert("bytes".into(), json!(shared.bytes.load(Ordering::Relaxed)));
    if let Some(err) = shared.error.lock().ok().and_then(|g| g.clone()) {
        meta.insert("error".into(), json!(err));
    }
    meta.insert("frames".into(), json!(infos));
    match serde_json::to_vec_pretty(&meta) {
        Ok(text) => {
            if let Err(e) = storage::write_atomic(&plan.sidecar, &text) {
                set_error(shared, &e);
                failed = true;
            }
        }
        Err(e) => {
            set_error(
                shared,
                &CaptureError::Failed(format!("sidecar serialisation failed: {e}")),
            );
            failed = true;
        }
    }
    failed
}
