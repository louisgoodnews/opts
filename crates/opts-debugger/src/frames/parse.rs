//! Parsers of the three frame-time inputs. Pure: they read `&str`, never files.

use super::Frame;
use crate::error::DebuggerError;

/// The values of an `OPTS_REFLECT_STATS` summary line, as the writer printed them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReflectSummary {
    /// Frames sampled after the warm-up (`frames=`).
    pub frames: u64,
    /// Mean frame time in milliseconds (`mean_ms=`).
    pub mean_ms: f64,
    /// Median frame time in milliseconds (`p50_ms=`).
    pub p50_ms: f64,
    /// 95th percentile in milliseconds (`p95_ms=`).
    pub p95_ms: f64,
    /// 99th percentile in milliseconds (`p99_ms=`).
    pub p99_ms: f64,
    /// Frames per second as the writer computed it (`fps=`).
    pub fps: f64,
}

/// The non-blank lines of `text`, with their 1-based line numbers.
fn lines(text: &str) -> impl Iterator<Item = (usize, &str)> {
    text.lines()
        .enumerate()
        .filter(|(_, row)| !row.trim().is_empty())
        .map(|(i, row)| (i + 1, row))
}

fn parse_error(format: &'static str, line: usize, reason: impl Into<String>) -> DebuggerError {
    DebuggerError::Parse {
        format,
        line,
        reason: reason.into(),
    }
}

/// Parses the `OPTS_DBG_FRAMES` CSV: frame time = field 2 (seconds), timestamp = field 1 (seconds).
pub fn parse_csv(text: &str) -> Result<Vec<Frame>, DebuggerError> {
    let mut frames = Vec::new();
    for (line, row) in lines(text) {
        let field = |i: usize| -> Result<f64, DebuggerError> {
            row.split(',')
                .nth(i)
                .and_then(|s| s.trim().parse::<f64>().ok())
                .ok_or_else(|| parse_error("csv", line, format!("field {} is not a number", i + 1)))
        };
        let elapsed_s = field(0)?;
        let delta_s = field(1)?;
        frames.push(Frame {
            index: frames.len() as u64,
            at_ms: elapsed_s * 1000.0,
            time_ms: delta_s * 1000.0,
        });
    }
    Ok(frames)
}

/// Reads `{"f":<frame>,"dt":<ms>,` at the start of a line; `None` if the shape differs.
fn jsonl_fields(row: &str) -> Option<(u64, f64)> {
    let rest = row.trim().strip_prefix("{\"f\":")?;
    let (frame, rest) = rest.split_once(',')?;
    let dt = rest.strip_prefix("\"dt\":")?.split_once(',')?.0;
    Some((frame.trim().parse().ok()?, dt.trim().parse().ok()?))
}

/// Parses the `OPTS_STREAM_TRACE` JSON lines: frame time = `dt` (ms), index = `f`. The timestamp
/// is the running sum of `dt`, because the line carries no time.
pub fn parse_jsonl(text: &str) -> Result<Vec<Frame>, DebuggerError> {
    let mut frames = Vec::new();
    let mut at_ms = 0.0;
    for (line, row) in lines(text) {
        let (index, dt_ms) = jsonl_fields(row).ok_or_else(|| {
            parse_error("jsonl", line, "expected {\"f\":<frame>,\"dt\":<ms>,\"s\":{...}}")
        })?;
        at_ms += dt_ms;
        frames.push(Frame {
            index,
            at_ms,
            time_ms: dt_ms,
        });
    }
    Ok(frames)
}

/// Looks up `key=<number>` among the whitespace-separated tokens of `row`.
fn value_of(row: &str, key: &str) -> Option<f64> {
    row.split_whitespace().find_map(|token| {
        token
            .split_once('=')
            .filter(|(name, _)| *name == key)
            .and_then(|(_, value)| value.parse().ok())
    })
}

/// Parses the first line of an `OPTS_REFLECT_STATS` file; the `diag` lines after it are ignored.
pub fn parse_reflect(text: &str) -> Result<ReflectSummary, DebuggerError> {
    let (line, row) = lines(text)
        .next()
        .ok_or_else(|| parse_error("reflect", 1, "the file is empty"))?;
    let need = |key: &str| {
        value_of(row, key)
            .ok_or_else(|| parse_error("reflect", line, format!("missing or bad `{key}=`")))
    };
    Ok(ReflectSummary {
        frames: need("frames")? as u64,
        mean_ms: need("mean_ms")?,
        p50_ms: need("p50_ms")?,
        p95_ms: need("p95_ms")?,
        p99_ms: need("p99_ms")?,
        fps: need("fps")?,
    })
}
