//! Language packs: a folder per language in the user data directory.
//!
//! ```text
//! <data>/lang/<code>/pack.json      identity of the pack (required)
//! <data>/lang/<code>/*.ftl          messages, Fluent subset (any file names)
//! <data>/lang/<code>/*.json         messages, interchange form (`opts-l10n/1` or a flat map)
//! ```
//!
//! Nothing here panics or returns an error to the caller: a pack that cannot be read or fails
//! its checks yields [`Issue`]s, and the messages that are fine are still used.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::check::{self, Issue, Reference, Severity, Stats};
use super::fluent::{self, Table};
use super::interchange;

mod load;
pub use load::{default_root, discover, load_pack_dir, Pack};

mod direction;
mod meta;
mod validate;
mod parse_meta;

pub use direction::*;
pub use meta::*;
use validate::*;
pub use parse_meta::*;

/// Largest string file read, in bytes.
pub const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
/// Most packs discovered in one folder.
pub const MAX_PACKS: usize = 64;
/// Most string files read per pack.
pub const MAX_FILES: usize = 64;
/// The licence expected of contributed text.
pub const EXPECTED_LICENSE: &str = "MIT OR Apache-2.0";






const FILE: &str = "pack.json";







#[cfg(test)]
mod tests;
