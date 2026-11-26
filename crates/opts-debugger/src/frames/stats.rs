//! Statistics of frame times: the summary, the worst frames and the windows. Pure.

use super::Frame;

/// The statistics of a set of frames. Times are in milliseconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Summary {
    /// Number of frames.
    pub frames: usize,
    /// Arithmetic mean of the frame times.
    pub mean_ms: f64,
    /// The frame time at the 50th percentile rank.
    pub median_ms: f64,
    /// The frame time at the 95th percentile rank.
    pub p95_ms: f64,
    /// The frame time at the 99th percentile rank.
    pub p99_ms: f64,
    /// Mean of the slowest `low_1_percent_frames` frames (the 1 % low).
    pub low_1_percent_ms: f64,
    /// How many frames the 1 % low averages: `ceil(frames / 100)`, at least 1.
    pub low_1_percent_frames: usize,
    /// The slowest frame.
    pub worst_ms: f64,
    /// Frames slower than 45 ms.
    pub over_45_ms: usize,
    /// Frames slower than 100 ms.
    pub over_100_ms: usize,
}

/// One row of the per-window table: the summary of `frames` consecutive frames.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowRow {
    /// The index of the first frame of the window.
    pub first_index: u64,
    /// The statistics of the window's frames.
    pub summary: Summary,
}

/// The value at percentile `p` (0.0 to 1.0) of an ascending, non-empty list: index `floor((n-1)*p)`.
fn rank(sorted: &[f64], p: f64) -> f64 {
    sorted[((sorted.len() - 1) as f64 * p) as usize]
}

/// The number of values above `limit` in an ascending list.
fn over(sorted: &[f64], limit: f64) -> usize {
    sorted.iter().filter(|&&t| t > limit).count()
}

/// The statistics of `frames`, or `None` when there are no frames.
pub fn summarize(frames: &[Frame]) -> Option<Summary> {
    if frames.is_empty() {
        return None;
    }
    let mut sorted: Vec<f64> = frames.iter().map(|f| f.time_ms).collect();
    sorted.sort_by(f64::total_cmp);
    let n = sorted.len();
    let low_frames = n.div_ceil(100).max(1);
    let slowest = &sorted[n - low_frames..];
    Some(Summary {
        frames: n,
        mean_ms: sorted.iter().sum::<f64>() / n as f64,
        median_ms: rank(&sorted, 0.5),
        p95_ms: rank(&sorted, 0.95),
        p99_ms: rank(&sorted, 0.99),
        low_1_percent_ms: slowest.iter().sum::<f64>() / low_frames as f64,
        low_1_percent_frames: low_frames,
        worst_ms: sorted[n - 1],
        over_45_ms: over(&sorted, 45.0),
        over_100_ms: over(&sorted, 100.0),
    })
}

/// The `n` slowest frames, slowest first. Frames with equal time keep their file order.
pub fn worst(frames: &[Frame], n: usize) -> Vec<Frame> {
    let mut slowest = frames.to_vec();
    slowest.sort_by(|a, b| b.time_ms.total_cmp(&a.time_ms));
    slowest.truncate(n);
    slowest
}

/// One row per window of `size` consecutive frames in file order; the last window may be shorter.
pub fn windows(frames: &[Frame], size: usize) -> Vec<WindowRow> {
    frames
        .chunks(size.max(1))
        .filter_map(|chunk| {
            Some(WindowRow {
                first_index: chunk.first()?.index,
                summary: summarize(chunk)?,
            })
        })
        .collect()
}
