//! The frame times since the last reset: the recorder, the summary and the memory figure.

use super::*;

/// Frame times since the last reset.
#[derive(Resource, Default)]
pub struct FrameTimes {
    /// Seconds per frame.
    pub samples: Vec<f32>,
}

pub(super) fn record_frame_time(time: Res<Time<Real>>, mut frames: ResMut<FrameTimes>) {
    if frames.samples.len() < 1_000_000 {
        frames.samples.push(time.delta_secs());
    }
}

/// Summary of the frame times since the last reset.
#[must_use]
pub fn frame_summary(samples: &[f32]) -> Value {
    if samples.is_empty() {
        return json!({ "frames": 0 });
    }
    let mut sorted = samples.to_vec();
    sorted.sort_by(f32::total_cmp);
    let n = sorted.len();
    let avg = sorted.iter().sum::<f32>() / n as f32;
    let slow = (n / 100).max(1);
    let low_1pct = sorted[n - slow..].iter().sum::<f32>() / slow as f32;
    let ms = |s: f32| (f64::from(s) * 1000.0 * 1000.0).round() / 1000.0;
    json!({
        "frames": n,
        "avg_ms": ms(avg),
        "median_ms": ms(sorted[n / 2]),
        "p99_ms": ms(sorted[(n * 99 / 100).min(n - 1)]),
        "max_ms": ms(sorted[n - 1]),
        "avg_fps": (1.0 / f64::from(avg) * 10.0).round() / 10.0,
        "low_1pct_fps": (1.0 / f64::from(low_1pct) * 10.0).round() / 10.0,
    })
}

pub(super) fn memory_kb() -> Value {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let field = |name: &str| {
        status
            .lines()
            .find(|l| l.starts_with(name))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|v| v.parse::<u64>().ok())
    };
    json!({ "rss_kb": field("VmRSS:"), "peak_rss_kb": field("VmHWM:") })
}
