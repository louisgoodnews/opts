//! The recording tick: why a take stops, and one frame of the take, run by the driver each frame.

use super::*;

pub(super) fn stop_reason(
    world: &World,
    a: &mut Active,
    now: f64,
    limits: &CaptureLimits,
) -> Option<&'static str> {
    let s = &a.shared;
    let elapsed_ms = ((now - a.started) * 1000.0) as u64;
    if a.stop_request {
        return Some("request");
    }
    if s.state.load(Ordering::Acquire) == FAILED {
        return Some("encoder_error");
    }
    if elapsed_ms >= a.duration_ms {
        return Some("duration");
    }
    if s.bytes.load(Ordering::Relaxed) >= a.max_bytes {
        return Some("size");
    }
    if now - a.last_disk_check >= 1.0 {
        a.last_disk_check = now;
        if let Some(state) = world.get_resource::<CaptureState>() {
            let free = state.root.as_deref().and_then(|r| (state.free_space)(r));
            if free.is_some_and(|f| f < limits.min_free_disk_mb * 1024 * 1024) {
                return Some("disk");
            }
        }
    }
    let fps = world.get_resource::<ControlState>().map_or(0.0, |c| c.fps);
    if fps > 0.0 {
        s.degraded
            .store(fps < limits.degrade_below_fps, Ordering::Relaxed);
        if fps < limits.stop_below_fps {
            let since = *a.low_fps_since.get_or_insert(now);
            if now - since >= limits.stop_after.as_secs_f64() {
                return Some("degraded");
            }
        } else {
            a.low_fps_since = None;
        }
    }
    let full = s.full_since_ms.load(Ordering::Relaxed);
    if full > 0 && s.ms().saturating_sub(full) >= limits.queue_full_after.as_millis() as u64 {
        return Some("degraded");
    }
    let last = s.last_delivery_ms.load(Ordering::Relaxed);
    if s.in_flight.load(Ordering::Relaxed) > 0
        && s.ms().saturating_sub(last) >= limits.no_delivery_after.as_millis() as u64
    {
        return Some("window");
    }
    None
}

pub(super) fn tick(
    world: &mut World,
    a: &mut Active,
    now: f64,
    limits: &CaptureLimits,
    backend: CaptureBackend,
) {
    let degraded = a.shared.degraded.load(Ordering::Relaxed);
    let interval = f64::from(a.fps.max(1)).recip() * if degraded { 2.0 } else { 1.0 };
    if now < a.next_due {
        return;
    }
    let missed = ((now - a.next_due) / interval).floor() as u64;
    a.shared.dropped.fetch_add(missed, Ordering::Relaxed);
    a.next_due += interval * (missed + 1) as f64;
    if a.shared.in_flight.load(Ordering::Relaxed) >= limits.max_in_flight {
        a.shared.dropped.fetch_add(1, Ordering::Relaxed);
        return;
    }
    let Some(tx) = a.tx.clone() else { return };
    let info = FrameInfo {
        index: a.requested,
        sim_time: world
            .get_resource::<Time<Virtual>>()
            .map_or(0.0, |t| (t.elapsed_secs_f64() * 1000.0).round() / 1000.0),
        frame: world.get_resource::<ControlState>().map_or(0, |c| c.frame),
        wall_ms: ((now - a.started) * 1000.0) as u64,
    };
    a.requested += 1;
    let shared = a.shared.clone();
    shared.in_flight.fetch_add(1, Ordering::Relaxed);
    let sink_shared = shared.clone();
    let requested_at = Instant::now();
    let sink = Box::new(move |image: Image| {
        let s = &sink_shared;
        s.in_flight.fetch_sub(1, Ordering::Relaxed);
        s.delivery_us
            .fetch_add(requested_at.elapsed().as_micros() as u64, Ordering::Relaxed);
        s.delivered.fetch_add(1, Ordering::Relaxed);
        s.last_delivery_ms.store(s.ms(), Ordering::Relaxed);
        match tx.try_send(Msg::Frame(image, info)) {
            Ok(()) => s.full_since_ms.store(0, Ordering::Relaxed),
            Err(TrySendError::Full(_)) => {
                s.dropped.fetch_add(1, Ordering::Relaxed);
                let _ = s.full_since_ms.compare_exchange(
                    0,
                    s.ms().max(1),
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                );
            }
            Err(TrySendError::Disconnected(_)) => {}
        }
    });
    if let Err(e) = (backend.request)(world, sink) {
        shared.in_flight.fetch_sub(1, Ordering::Relaxed);
        shared.dropped.fetch_add(1, Ordering::Relaxed);
        if matches!(e, CaptureError::NotReady(_)) {
            a.forced_reason = Some("window");
        }
    }
}
