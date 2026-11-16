//! # OPTS Catalogue (`opts-catalogue`)
//!
//! A SQLite cache of an OMSI 2 installation: every directory and file, the discovered content
//! entities (vehicles, maps, scenery objects, …), their display metadata and their references.
//! Menus and loaders query it instead of walking 600,000+ files on every launch.
//!
//! Design: `docs/src/architecture/catalogue-schema.md` (schema, staleness) and
//! `docs/src/architecture/playability-rules.md` (which vehicles are player-selectable).
//!
//! Staleness: the catalogue is stale when it was never completed, was built for a different
//! root, or is older than the caller's maximum age (24 h in the game). A stale catalogue is
//! rebuilt by [`discover`], which writes inside one transaction so readers keep seeing the
//! previous snapshot until the new one commits.

#![warn(missing_docs)]

pub mod extract;
pub mod scan;
pub mod schema;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};

mod error;
mod handle;
mod lookup;
mod queries;
mod records;
mod relocation;
mod status;

pub use error::{Error, Result};
pub use handle::Catalogue;
pub use records::{MapRecord, VehicleRecord};
pub use status::{KindCounts, Phase, Staleness, Summary};

use handle::now_secs;

/// Version of the scan logic. Bumped when what a scan records changes (new reference kinds,
/// rerouting) so that databases from older builds are rebuilt even if they are recent.
pub const SCAN_VERSION: i64 = 4;

mod discover;
pub use discover::discover;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod relocation_tests;
