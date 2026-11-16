//! Screenshot, capture, command-list and output commands of the table, in protocol order.

use crate::commands::{c, CommandSpec};

pub(super) const PART: &[CommandSpec] = &[
    c(
        "capture.burst",
        "capture",
        "Capture N frames at a fixed interval as a PNG sequence",
        "count:i@0 interval_ms:i@1 format:s[png|jpeg]? quality:i? dir:s? metadata:b?",
    )
    .since("1.5")
    .result("`{dir, paths:[..], frames:[{index, path, sim_time, frame, wall_ms}], dropped, manifest}`")
    .doc("1-300 frames, `interval_ms` 16-60000, and `count` x `interval_ms` at most 90 000 ms. Frames are written to `dir` (a relative name below the captures directory; default `{datetime}_{session_id}_{counter}_burst`) as `0001.png`, … with a `manifest.json` carrying the shared metadata and one entry per frame. Replies when the last frame is written; a frame the renderer does not deliver within 2 s counts in `dropped`. Only one capture job (burst or recording) runs at a time (`1006`); `1001` without a renderer or while minimized, `1002` for a `dir` outside the captures directory."),
    c(
        "capture.record.start",
        "capture",
        "Start a bounded recording of the application window",
        "name:s? format:s[av1|png_sequence|h264]? fps:i? duration_ms:i? max_size_mb:i? quality:i? scale:f?",
    )
    .since("1.5")
    .result("`{recording_id, path, format, fps, effective_limits:{duration_ms, max_size_mb, min_free_disk_mb}}`")
    .doc("Returns at once; the recording runs in the background and is off unless started. `format` is `av1` (default; AV1 in an `.ivf` file, encoded by rav1e) or `png_sequence` (a directory of PNGs, ends early at the size cap); `h264` needs an optional build feature. `fps` 1-60 (default 30), `duration_ms` default 60 000 (the `opts-cli` default is 30 000) up to 30 min, `max_size_mb` default 512 up to 2048, `quality` 1-100, `scale` 0.25-1, rounded to a whole reduction (1, 1/2, 1/3, 1/4) and **0.5 by default**: the encoder is a pure-Rust AV1 build without assembly and measures about 9 fps at 1920x1013, 23 fps at half and 29 fps at a quarter size; frames it cannot keep up with are dropped (`dropped`, and `timing_ms` in `status` shows where the time goes). It also stops below 1 GiB free disk space, when the frame rate stays under 30 for 5 s, or when the renderer stops delivering frames. `name` is a relative stem below the captures directory. A sidecar `<file>.json` lists every frame with its simulation time. `1006` while another job runs, `1008` when the disk is nearly full, `1001` without a renderer."),
    c(
        "capture.record.stop",
        "capture",
        "Stop a recording and return its result",
        "recording_id:s@0=id",
    )
    .since("1.5")
    .result("`{recording_id, state, path, format, duration_ms, frame_count, dropped, bytes, stopped_by}`")
    .doc("`stopped_by` is `request`, `duration`, `size`, `disk`, `degraded`, `encoder_error` or `window`. Waits (at most 10 s) until the file is closed; stopping a finished recording returns its stored result. `1004` for an unknown id."),
    c(
        "capture.record.status",
        "capture",
        "Progress of a recording",
        "recording_id:s@0=id",
    )
    .since("1.5")
    .result("`{recording_id, state, frame_count, elapsed_ms, dropped, bytes, fps_measured, degraded, stopped_by?}`")
    .doc("`state` is `recording`, `finishing`, `finished` or `failed`; a finished recording returns its final result (the last 8 are kept). Never waits."),
    c(
        "capture.prune",
        "capture",
        "Delete old captures written by this subsystem (dry run unless told otherwise)",
        "older_than_days:i@0 dry_run:b?",
    )
    .since("1.5")
    .result("`{dry_run, items:[{path, bytes}], bytes}`")
    .doc("Only files with a metadata sidecar and burst or recording directories with a `manifest.json` are candidates, judged by their modification time; other files in the captures directory are never touched. `dry_run` defaults to true."),
    // ---- protocol 1.1: discovery -------------------------------------------------------------
    c(
        "commands",
        "meta",
        "Every command with its argument schema, result and documentation",
        "group:s?",
    )
    .since("1.1")
    .result("`{protocol_version, count, commands:[{method, group, summary, since, args, result, doc, debug, implemented}]}`")
    .doc("The machine-readable form of the generated reference. `args` entries are `{name, type, required, position, aliases, values?}`; `debug` marks commands that skip the input layer on purpose."),
    // ---- protocol 1.1: UI -------------------------------------------------------------------
];
