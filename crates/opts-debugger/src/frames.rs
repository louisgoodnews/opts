//! Frame-time reader (ISS-096 stage 4, the `frames` tool).
//!
//! Reads a frame-time file of a run and prints its summary: the frame count, mean, median, p95,
//! p99, the 1 % low, the frames over 45 ms and 100 ms, the worst frames, and one row per window of
//! frames. Three inputs are read; each names the field the frame time comes from:
//!
//! - `csv`: the `OPTS_DBG_FRAMES=<file>` file of `opts-game` (`in_game/session.rs`,
//!   `debug_frame_log`). One line per frame, no header: `elapsed_s,delta_s,x,z,speed,steps`. The
//!   frame time is field 2, `delta_s`, in seconds; the timestamp is field 1, the elapsed real time.
//!   The frame index is the 0-based position among the frame lines.
//! - `jsonl`: the `OPTS_STREAM_TRACE=<file>` file (`in_game/session.rs`, `stream_trace_frame`;
//!   built with the `stream-trace` feature). One JSON object per frame,
//!   `{"f":<frame>,"dt":<ms>,"s":{...}}`. The frame time is `dt`, in milliseconds, and the index
//!   is `f`. The line has no time, so the timestamp is the running sum of `dt`. Only `f` and `dt`
//!   are read; the span table `s` is ignored.
//! - `reflect`: the `OPTS_REFLECT_STATS=<file>` summary (`opts-bevy`, `reflections/stats.rs`). It
//!   holds no frame times, so its `frames`, `mean_ms`, `p50_ms`, `p95_ms`, `p99_ms` and `fps`
//!   values are printed as written. The worst frames, the windows, the 1 % low and the counts over
//!   45 and 100 ms need the per-frame formats.
//!
//! Statistics (for `csv` and `jsonl`): a percentile is the sorted time at index `floor((n-1)*p)`,
//! the rule of `stats.rs`, so the numbers agree with the reflection summary. The 1 % low is the
//! mean of the slowest `ceil(n / 100)` frames (at least one frame), and the printed report says
//! so. The p99 is the frame time at the 99th percentile rank, not a mean.
//!
//! The library holds the pure part: parsing, statistics and rendering, with no file I/O. The
//! binary reads the file and calls [`frames_report`].

mod parse;
mod render;
mod stats;

use crate::error::DebuggerError;

pub use parse::{parse_csv, parse_jsonl, parse_reflect, ReflectSummary};
pub use stats::{summarize, windows, worst, Summary, WindowRow};

/// The input format of a frame-time file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// The `OPTS_DBG_FRAMES` CSV (one line per frame).
    Csv,
    /// The `OPTS_STREAM_TRACE` JSON lines (one object per frame).
    Jsonl,
    /// The `OPTS_REFLECT_STATS` summary (no per-frame data).
    Reflect,
}

/// One frame of a run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    /// The frame number as the writer numbers it (or the 0-based line position for the CSV).
    pub index: u64,
    /// The time of the frame, in milliseconds since the start of the run (see the module doc).
    pub at_ms: f64,
    /// The frame time in milliseconds.
    pub time_ms: f64,
}

/// Parses `text` in `format` and returns the plain-text report.
///
/// `worst_n` is the number of worst frames to list and `window` the number of frames per window
/// row (a value of 0 is read as 1). For [`Format::Reflect`] both are ignored, because that format
/// holds no frames. Returns [`DebuggerError::NoFrames`] for a frame file with no frame.
pub fn frames_report(
    text: &str,
    format: Format,
    worst_n: usize,
    window: usize,
) -> Result<String, DebuggerError> {
    if format == Format::Reflect {
        return Ok(render::render_reflect(&parse_reflect(text)?));
    }
    let frames = match format {
        Format::Csv => parse_csv(text)?,
        _ => parse_jsonl(text)?,
    };
    let summary = summarize(&frames).ok_or(DebuggerError::NoFrames)?;
    let window = window.max(1);
    Ok(render::render_frames(
        &summary,
        &worst(&frames, worst_n),
        &windows(&frames, window),
        window,
    ))
}
