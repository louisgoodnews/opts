//! Plain-text rendering of the `frames` report. Pure: it returns the text and the binary prints it.

use std::fmt::Write;

use super::{Frame, ReflectSummary, Summary, WindowRow};

/// The report of a per-frame input: summary, worst frames, then the window rows.
pub(super) fn render_frames(
    summary: &Summary,
    worst: &[Frame],
    windows: &[WindowRow],
    window: usize,
) -> String {
    let mut out = String::new();
    render_summary(&mut out, summary);
    render_worst(&mut out, worst);
    render_windows(&mut out, windows, window);
    out
}

fn render_summary(out: &mut String, s: &Summary) {
    let _ = writeln!(out, "frames:      {}", s.frames);
    let _ = writeln!(out, "mean:        {:.3} ms ({:.1} fps)", s.mean_ms, 1000.0 / s.mean_ms);
    let _ = writeln!(out, "median:      {:.3} ms", s.median_ms);
    let _ = writeln!(out, "p95:         {:.3} ms", s.p95_ms);
    let _ = writeln!(out, "p99:         {:.3} ms (frame time at the 99th percentile rank)", s.p99_ms);
    let _ = writeln!(
        out,
        "1 % low:     {:.3} ms (mean of the slowest {} frames, ceil(1 % of {}))",
        s.low_1_percent_ms, s.low_1_percent_frames, s.frames
    );
    let _ = writeln!(out, "over 45 ms:  {} frames", s.over_45_ms);
    let _ = writeln!(out, "over 100 ms: {} frames", s.over_100_ms);
}

fn render_worst(out: &mut String, worst: &[Frame]) {
    let _ = writeln!(out, "\nworst {} frames:", worst.len());
    let _ = writeln!(out, "  {:>8}  {:>10}  {:>10}", "index", "at (s)", "frame (ms)");
    for f in worst {
        let _ = writeln!(
            out,
            "  {:>8}  {:>10.3}  {:>10.3}",
            f.index,
            f.at_ms / 1000.0,
            f.time_ms
        );
    }
}

fn render_windows(out: &mut String, windows: &[WindowRow], window: usize) {
    let _ = writeln!(out, "\nwindows of {window} frames (the last may be shorter):");
    let _ = writeln!(
        out,
        "  {:>8}  {:>6}  {:>9}  {:>9}  {:>9}  {:>9}",
        "first", "frames", "mean", "median", "p95", "worst"
    );
    for w in windows {
        let s = &w.summary;
        let _ = writeln!(
            out,
            "  {:>8}  {:>6}  {:>9.3}  {:>9.3}  {:>9.3}  {:>9.3}",
            w.first_index, s.frames, s.mean_ms, s.median_ms, s.p95_ms, s.worst_ms
        );
    }
}

/// The report of an `OPTS_REFLECT_STATS` summary: its values as written, and what it lacks.
pub(super) fn render_reflect(r: &ReflectSummary) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "frames:      {}", r.frames);
    let _ = writeln!(out, "mean:        {:.3} ms", r.mean_ms);
    let _ = writeln!(out, "p50:         {:.3} ms", r.p50_ms);
    let _ = writeln!(out, "p95:         {:.3} ms", r.p95_ms);
    let _ = writeln!(out, "p99:         {:.3} ms", r.p99_ms);
    let _ = writeln!(out, "fps:         {:.1} (as written)", r.fps);
    let _ = writeln!(
        out,
        "\nOPTS_REFLECT_STATS writes a summary only: the 1 % low, the worst frames, the windows and\n\
         the counts over 45 and 100 ms need the per-frame formats (csv or jsonl)."
    );
    out
}
