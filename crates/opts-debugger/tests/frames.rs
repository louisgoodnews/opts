//! Tests of the `frames` reader on small in-memory inputs with hand-computed statistics.
//!
//! Percentile rule: sorted index `floor((n-1)*p)`. 1 % low: mean of the slowest `ceil(n/100)`.

use opts_debugger::frames::{
    frames_report, parse_csv, parse_jsonl, parse_reflect, summarize, windows, worst, Format, Frame,
};
use opts_debugger::DebuggerError;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

/// Frames with the given times in ms; index = position, timestamp = running sum.
fn frames_of(times_ms: &[f64]) -> Vec<Frame> {
    let mut at = 0.0;
    times_ms
        .iter()
        .enumerate()
        .map(|(i, &t)| {
            at += t;
            Frame {
                index: i as u64,
                at_ms: at,
                time_ms: t,
            }
        })
        .collect()
}

/// A CSV input with the given frame times in seconds (fields 1 and 2 are the ones read).
fn csv_of(deltas_s: &[f64]) -> String {
    deltas_s
        .iter()
        .map(|d| format!("0.00000,{d:.5},1.0000,2.0000,0.000,0\n"))
        .collect()
}

#[test]
fn summary_of_five_frames_is_hand_computed() {
    // Sorted: 10, 20, 30, 40, 1000. Mean 220. Rank floor(4*p): p50 -> 2, p95 -> 3, p99 -> 3.
    let s = summarize(&frames_of(&[10.0, 20.0, 30.0, 40.0, 1000.0])).expect("frames");
    assert_eq!(s.frames, 5);
    assert!(close(s.mean_ms, 220.0));
    assert!(close(s.median_ms, 30.0));
    assert!(close(s.p95_ms, 40.0));
    assert!(close(s.p99_ms, 40.0));
    // ceil(5/100) = 1 frame: the slowest one.
    assert_eq!(s.low_1_percent_frames, 1);
    assert!(close(s.low_1_percent_ms, 1000.0));
    assert!(close(s.worst_ms, 1000.0));
    assert_eq!((s.over_45_ms, s.over_100_ms), (1, 1));
}

#[test]
fn one_percent_low_averages_two_frames_of_two_hundred() {
    // 198 frames of 10 ms, then 50 and 70 ms. Mean 10.5. Rank floor(199*p): p50 -> 99,
    // p95 -> 189, p99 -> 197: all 10 ms. ceil(200/100) = 2 slowest: (50 + 70) / 2 = 60.
    let mut times = vec![10.0; 198];
    times.extend([50.0, 70.0]);
    let s = summarize(&frames_of(&times)).expect("frames");
    assert_eq!(s.frames, 200);
    assert!(close(s.mean_ms, 10.5));
    assert!(close(s.median_ms, 10.0));
    assert!(close(s.p95_ms, 10.0));
    assert!(close(s.p99_ms, 10.0));
    assert_eq!(s.low_1_percent_frames, 2);
    assert!(close(s.low_1_percent_ms, 60.0));
    assert_eq!((s.over_45_ms, s.over_100_ms), (2, 0));
}

#[test]
fn csv_reads_delta_seconds_as_frame_time() {
    let frames = parse_csv(&csv_of(&[0.010, 0.020, 0.030])).expect("csv");
    let times: Vec<f64> = frames.iter().map(|f| f.time_ms).collect();
    let indices: Vec<u64> = frames.iter().map(|f| f.index).collect();
    assert!(times.iter().zip([10.0, 20.0, 30.0]).all(|(a, b)| close(*a, b)));
    assert_eq!(indices, vec![0, 1, 2]);
}

#[test]
fn jsonl_reads_dt_and_accumulates_time() {
    let text = concat!(
        "{\"f\":0,\"dt\":0.000,\"s\":{}}\n",
        "{\"f\":1,\"dt\":16.500,\"s\":{\"tile.parse\":[2,3.000,2.000]}}\n",
        "{\"f\":2,\"dt\":33.000,\"s\":{}}\n",
    );
    let frames = parse_jsonl(text).expect("jsonl");
    assert_eq!(frames.iter().map(|f| f.index).collect::<Vec<_>>(), vec![0, 1, 2]);
    assert!(close(frames[1].time_ms, 16.5));
    assert!(close(frames[2].time_ms, 33.0));
    assert!(close(frames[2].at_ms, 49.5));
}

#[test]
fn reflect_summary_is_read_as_written() {
    let text = "frames=600 mean_ms=16.667 p50_ms=16.000 p95_ms=20.000 p99_ms=25.500 fps=60.0 rss_mib=812\ndiag render/x avg=1.0\n";
    let r = parse_reflect(text).expect("reflect");
    assert_eq!(r.frames, 600);
    assert!(close(r.mean_ms, 16.667));
    assert!(close(r.p50_ms, 16.0));
    assert!(close(r.p95_ms, 20.0));
    assert!(close(r.p99_ms, 25.5));
    assert!(close(r.fps, 60.0));
}

#[test]
fn windows_of_two_frames_over_five() {
    // [10, 20] [30, 40] [1000]: window means 15, 35, 1000; medians 10, 30, 1000 (rank 0 of 2).
    let rows = windows(&frames_of(&[10.0, 20.0, 30.0, 40.0, 1000.0]), 2);
    let firsts: Vec<u64> = rows.iter().map(|w| w.first_index).collect();
    let counts: Vec<usize> = rows.iter().map(|w| w.summary.frames).collect();
    assert_eq!(firsts, vec![0, 2, 4]);
    assert_eq!(counts, vec![2, 2, 1]);
    assert!(close(rows[0].summary.mean_ms, 15.0));
    assert!(close(rows[0].summary.median_ms, 10.0));
    assert!(close(rows[0].summary.worst_ms, 20.0));
    assert!(close(rows[1].summary.mean_ms, 35.0));
    assert!(close(rows[1].summary.median_ms, 30.0));
    assert!(close(rows[2].summary.worst_ms, 1000.0));
}

#[test]
fn worst_is_slowest_first_and_ties_keep_file_order() {
    let frames = frames_of(&[10.0, 20.0, 30.0, 40.0, 1000.0]);
    let top: Vec<u64> = worst(&frames, 2).iter().map(|f| f.index).collect();
    assert_eq!(top, vec![4, 3]);
    let tie = frames_of(&[5.0, 5.0]);
    assert_eq!(worst(&tie, 1)[0].index, 0);
    assert!(worst(&frames, 0).is_empty());
}

#[test]
fn malformed_lines_name_their_line_number() {
    let csv = parse_csv("0.01000,0.01000,1,2,0,0\n0.02000,abc,1,2,0,0\n");
    assert!(matches!(csv, Err(DebuggerError::Parse { format: "csv", line: 2, .. })));
    let jsonl = parse_jsonl("not json\n");
    assert!(matches!(jsonl, Err(DebuggerError::Parse { format: "jsonl", line: 1, .. })));
    let reflect = parse_reflect("mean_ms=1.0\n");
    assert!(matches!(reflect, Err(DebuggerError::Parse { format: "reflect", .. })));
}

#[test]
fn empty_input_has_no_frames() {
    let report = frames_report("", Format::Csv, 10, 600);
    assert!(matches!(report, Err(DebuggerError::NoFrames)));
}

#[test]
fn report_states_the_low_and_the_rows() {
    let text = csv_of(&[0.010, 0.020, 0.030, 0.040, 1.000]);
    let report = frames_report(&text, Format::Csv, 2, 2).expect("report");
    assert!(report.contains("median:      30.000 ms"));
    assert!(report.contains("1 % low:     1000.000 ms"));
    assert!(report.contains("mean of the slowest 1 frames"));
    assert!(report.contains("windows of 2 frames"));
}
