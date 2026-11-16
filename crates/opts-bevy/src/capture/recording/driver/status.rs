//! The recording status as JSON: the result of a finished take and the live status with its timing.

use super::*;

pub(super) fn result_json(a: &Active, state: &str) -> Value {
    let s = &a.shared;
    let mut v = json!({
        "recording_id": a.id,
        "state": state,
        "path": a.output.display().to_string(),
        "format": a.format.name(),
        "fps": a.fps,
        "duration_ms": a.stopped_after_ms,
        "frame_count": s.frames.load(Ordering::Relaxed),
        "dropped": s.dropped.load(Ordering::Relaxed),
        "bytes": s.bytes.load(Ordering::Relaxed),
        "stopped_by": a.stopped_by,
        "timing_ms": timing_json(s),
    });
    if let Some(e) = s.error.lock().ok().and_then(|g| g.clone()) {
        v["error"] = json!(e);
    }
    v
}

/// The status payload of the active recording.
#[must_use]
pub fn status_json(a: &Active, now: f64) -> Value {
    let s = &a.shared;
    let elapsed_ms = ((now - a.started) * 1000.0) as u64;
    let state = if a.capturing() {
        "recording"
    } else {
        "finishing"
    };
    let frames = s.frames.load(Ordering::Relaxed);
    json!({
        "recording_id": a.id,
        "state": state,
        "frame_count": frames,
        "elapsed_ms": elapsed_ms,
        "dropped": s.dropped.load(Ordering::Relaxed),
        "bytes": s.bytes.load(Ordering::Relaxed),
        "fps_measured": if elapsed_ms > 0 { (frames as f64 * 1000.0 / elapsed_ms as f64 * 10.0).round() / 10.0 } else { 0.0 },
        "degraded": s.degraded.load(Ordering::Relaxed),
        "stopped_by": a.stopped_by,
        "timing_ms": timing_json(s),
    })
}

/// Average cost per frame of each stage, in milliseconds: the renderer's readback, the
/// conversion and the encoder. Diagnoses a recording that drops frames.
fn timing_json(s: &Shared) -> Value {
    let avg = |total: u64, n: u64| {
        if n == 0 {
            0.0
        } else {
            (total as f64 / n as f64 / 100.0).round() / 10.0
        }
    };
    let encoded = s.frames.load(Ordering::Relaxed);
    json!({
        "delivery": avg(s.delivery_us.load(Ordering::Relaxed), s.delivered.load(Ordering::Relaxed)),
        "convert": avg(s.convert_us.load(Ordering::Relaxed), encoded),
        "encode": avg(s.encode_us.load(Ordering::Relaxed), encoded),
    })
}
