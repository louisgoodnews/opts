//! The derived cache of generated block-compressed mip chains (M-0058 phase C).
//!
//! One file per texture in `<cache dir>/opts/textures/` (`~/.cache/opts/textures/` on Linux):
//! `<sha256 of format, size and level-0 bytes>-g<generator version>.mip`, holding a small header
//! (magic, generator version, format, size, level count, payload length and checksum) and the
//! chain. It is derived data on the user's machine only, never distributed. Files are written
//! to a unique temporary name and renamed, so concurrent writers leave one valid file; a short,
//! corrupt or mismatching file is ignored and rebuilt. A hit touches the file's modification
//! time, and [`ChainCache::prune`] removes files of other generator versions and then the least
//! recently used ones until the cache fits its cap (default 4 GiB). The game prunes once, on a
//! background thread, when the cache is first used.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::OnceLock;

use bevy::render::render_resource::TextureFormat;
use sha2::{Digest, Sha256};

use super::dds_chain::chain_bytes;
use super::full_chain_levels;
mod key;
mod parse;
mod store;

pub use key::ChainKey;
pub use store::{global, ChainCache};

/// Default size cap of the cache.
pub const DEFAULT_CAP_BYTES: u64 = 4 << 30;

const MAGIC: &[u8; 8] = b"OPTSBCM1";
const HEADER: usize = 8 + 4 * 5 + 8 + 8;
#[cfg(test)]
#[path = "chain_cache/tests.rs"]
mod tests;
