//! # 🪵 `opts-logging` — Unified Diagnostics Subsystem for OPTS
//!
//! Foundational logging and diagnostic infrastructure for the Open Public Transport Simulator.
//!
//! `opts-logging` provides:
//! - **Non-blocking asynchronous writers:** Thread-safe, non-blocking disk and console appenders
//!   preventing logging I/O from stalling simulation loops or rendering frames.
//! - **Subsystem file routing:** Routes log events based on module targets (`opts_core`, `opts_sim`,
//!   `opts_bevy`, `opts_gui`, `opts_game`, `opts_tools`) to dedicated log files as well as a centralized `opts.log`.
//! - **ANSI color formatting:** Clean badges, compact millisecond timestamps, and inline span hierarchies.
//! - **Rolling log rotation & retention:** Daily/hourly rotation and automated pruning of old log files.
//! - **Post-mortem crash telemetry:** Panic hook writing full stack backtraces and crash reports to `panics.log`.
//!
//! ## Quick Start
//!
//! ```no_run
//! use opts_logging::prelude::*;
//!
//! fn main() -> Result<(), LogError> {
//!     // Initialize logging with default options (holds non-blocking worker threads)
//!     let _guard = opts_logging::init_default()?;
//!
//!     tracing::info!("OPTS simulation engine initialized");
//!     Ok(())
//! }
//! ```
//!
//! ## Invariant Boundaries
//!
//! `opts-logging` is a Layer 0 crate. It **never** depends on any internal OPTS crate.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod config;
pub mod error;
pub mod guard;
pub mod init;
pub mod layer;
pub mod panic;
pub mod prelude;
pub mod stream_trace;
pub mod rotation;

pub use config::{
    AnsiMode, ConsoleConfig, ConsoleVerbosity, FilesConfig, LogConfig, LogFormat, RetentionPolicy,
    RotationPolicy,
};
pub use error::{LogError, LogResult};
pub use guard::LogGuard;
pub use init::{init, init_default, init_from_env, set_global_level, set_level};
pub use layer::ring::{recent as recent_logs, LogQuery, LogRecord};
pub use panic::install_panic_hook;
pub use rotation::apply_retention;
