//! The error type of the library.
//!
//! Every fallible function of the tools returns [`DebuggerError`].

/// An error of the development toolkit.
#[derive(Debug, thiserror::Error)]
pub enum DebuggerError {
    /// A tool was asked for before it is implemented (the skeleton stage).
    #[error("opts-debugger: the {0} tool is not implemented yet")]
    NotImplemented(&'static str),
}
