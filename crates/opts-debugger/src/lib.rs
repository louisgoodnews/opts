//! OPTS development toolkit: the tools a developer uses to read and debug a run.
//!
//! This crate is not part of the game. No game crate depends on it, and it is not shipped with
//! the game. The tools, one per ISS-096 stage, are:
//!
//! - [`frames`]: frame-time summaries of a run (the `OPTS_DBG_FRAMES` CSV, the `OPTS_STREAM_TRACE`
//!   lines and the `OPTS_REFLECT_STATS` summary).
//! - [`timing`]: the heaviest systems per window of an `OPTS_SYSTEM_TIMING` log.
//! - [`bundle`]: one folder with the logs, crash reports, timing and frame files and screenshots
//!   of a run, and a manifest.
//! - [`trace`]: an offline reader of a `scene-trace-*.jsonl` file.
//! - [`client`]: the crate's own client to the control protocol.
//!
//! The `frames` tool is implemented (ISS-096 stage 4); the other tools are still skeletons.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub(crate) mod bundle;
pub(crate) mod client;
pub mod frames;
pub(crate) mod timing;
pub(crate) mod trace;

pub mod error;

pub use error::DebuggerError;
