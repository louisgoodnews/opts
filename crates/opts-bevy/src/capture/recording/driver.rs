//! Per-frame driver and stop/degradation logic for continuous recording.

use super::*;

mod status;
mod tick;

pub use status::*;
use tick::*;

/// Per-frame driver: enforces the limits, captures on schedule and collects finished jobs.
pub fn drive(world: &mut World) {
    if world
        .get_resource::<Recorder>()
        .is_none_or(|r| r.active.is_none())
    {
        return;
    }
    let limits = world
        .get_resource::<CaptureLimits>()
        .cloned()
        .unwrap_or_default();
    let backend = world
        .get_resource::<CaptureBackend>()
        .copied()
        .unwrap_or_default();
    let now = world
        .resource::<Time<bevy::time::Real>>()
        .elapsed_secs_f64();
    world.resource_scope(|world, mut rec: bevy::prelude::Mut<Recorder>| {
        let Some(a) = rec.active.as_mut() else { return };
        if a.capturing() {
            let reason = a
                .forced_reason
                .take()
                .or_else(|| stop_reason(world, a, now, &limits));
            if let Some(reason) = reason {
                a.stopped_by = Some(reason);
                a.stopped_after_ms = ((now - a.started) * 1000.0) as u64;
                a.tx = None;
            } else {
                tick(world, a, now, &limits, backend);
            }
            return;
        }
        let state = a.shared.state.load(Ordering::Acquire);
        if state >= FINISHED {
            if let Some(h) = a.handle.take() {
                let _ = h.join();
            }
            let label = if state == FAILED {
                "failed"
            } else {
                "finished"
            };
            let result = result_json(a, label);
            let id = a.id.clone();
            rec.active = None;
            world
                .get_resource_or_insert_with(CaptureJobs::default)
                .finish(Finished { id, result });
        }
    });
}
