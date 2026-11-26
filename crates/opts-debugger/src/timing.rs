//! Timing-log reader (ISS-096 stage 4, the `timing` tool).
//!
//! Will read a log written with `OPTS_SYSTEM_TIMING` and rank the systems of each window by busy
//! time: total, runs and longest run. The line format is the one `opts-logging` writes.
