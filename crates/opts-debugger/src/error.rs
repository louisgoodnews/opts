//! The error type of the library.
//!
//! Every fallible function of the tools returns [`DebuggerError`]. The library performs no file
//! I/O: the [`DebuggerError::Read`] variant only carries an error that the binary got from reading
//! a file.

use std::path::PathBuf;

/// An error of the development toolkit.
#[derive(Debug, thiserror::Error)]
pub enum DebuggerError {
    /// The binary could not read an input file.
    #[error("opts-debugger: cannot read {path}: {source}")]
    Read {
        /// The file that could not be read.
        path: PathBuf,
        /// The reason the read failed.
        source: std::io::Error,
    },
    /// A line of an input does not have the shape its format requires.
    #[error("opts-debugger: {format} line {line}: {reason}")]
    Parse {
        /// The input format, as the tool names it (`csv`, `jsonl`, `reflect`).
        format: &'static str,
        /// The 1-based line number in the input.
        line: usize,
        /// What is wrong with the line.
        reason: String,
    },
    /// The input holds no frame, so there is nothing to summarise.
    #[error("opts-debugger: the input holds no frames")]
    NoFrames,
}
