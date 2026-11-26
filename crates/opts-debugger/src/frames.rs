//! Frame-time reader (ISS-096 stage 4, the `frames` tool).
//!
//! Will read a frame file of a run (the `OPTS_DBG_FRAMES` CSV, the `OPTS_STREAM_TRACE` JSON lines
//! or the `OPTS_REFLECT_STATS` summary) and report the count, mean, median, p95, p99, the 1 %
//! low, the frames over 45 ms and 100 ms, and the worst frames, per window of the run.
