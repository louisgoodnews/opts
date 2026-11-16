//! Where captures go: the captures directory, name validation, default names and atomic writes.
//!
//! Every name that reaches the file system passes through [`resolve_name`] or
//! [`resolve_explicit`]; both guarantee the result is inside the canonical captures root.

mod names;
mod resolve;
mod state;
mod write;

pub use names::validate_relative;
pub use resolve::{resolve_dir, resolve_explicit, resolve_name};
pub use state::{default_root, free_bytes, CaptureState};
pub use write::write_atomic;

#[cfg(test)]
mod tests;
