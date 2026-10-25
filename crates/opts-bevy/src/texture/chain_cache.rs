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

/// Default size cap of the cache.
pub const DEFAULT_CAP_BYTES: u64 = 4 << 30;

const MAGIC: &[u8; 8] = b"OPTSBCM1";
const HEADER: usize = 8 + 4 * 5 + 8 + 8;

/// Identity of one cached chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainKey {
    hash: String,
    format: u32,
    width: u32,
    height: u32,
    generator: u32,
}

impl ChainKey {
    /// The key of a texture: its format, size and level-0 bytes, and the generator version.
    #[must_use]
    pub fn new(
        format: TextureFormat,
        width: u32,
        height: u32,
        level0: &[u8],
        generator: u32,
    ) -> Self {
        let format = format_code(format);
        let mut hasher = Sha256::new();
        hasher.update(format.to_le_bytes());
        hasher.update(width.to_le_bytes());
        hasher.update(height.to_le_bytes());
        hasher.update(level0);
        let hash = hasher
            .finalize()
            .iter()
            .take(16)
            .map(|b| format!("{b:02x}"))
            .collect();
        Self {
            hash,
            format,
            width,
            height,
            generator,
        }
    }

    fn file_name(&self) -> String {
        format!("{}-g{}.mip", self.hash, self.generator)
    }

    fn block_bytes(&self) -> usize {
        if self.format == 1 {
            8
        } else {
            16
        }
    }
}

fn format_code(format: TextureFormat) -> u32 {
    match format {
        TextureFormat::Bc1RgbaUnormSrgb | TextureFormat::Bc1RgbaUnorm => 1,
        TextureFormat::Bc2RgbaUnormSrgb | TextureFormat::Bc2RgbaUnorm => 2,
        TextureFormat::Bc3RgbaUnormSrgb | TextureFormat::Bc3RgbaUnorm => 3,
        _ => 0,
    }
}

/// FNV-1a over the payload: catches truncated or damaged files, not attacks.
fn checksum(data: &[u8]) -> u64 {
    data.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// A directory of cached chains with a size cap.
#[derive(Debug, Clone)]
pub struct ChainCache {
    dir: PathBuf,
    cap_bytes: u64,
    generator: u32,
}

impl ChainCache {
    /// A cache in `dir` (created on the first write) holding chains of `generator`.
    #[must_use]
    pub fn new(dir: PathBuf, cap_bytes: u64, generator: u32) -> Self {
        Self {
            dir,
            cap_bytes,
            generator,
        }
    }

    /// The cache directory.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The cached chain of `key` and its level count, when a valid file exists whose level 0
    /// equals `level0`. A hit marks the file as recently used.
    #[must_use]
    pub fn get(&self, key: &ChainKey, level0: &[u8]) -> Option<(Vec<u8>, u32)> {
        let path = self.dir.join(key.file_name());
        let bytes = fs::read(&path).ok()?;
        let (chain, levels) = parse(&bytes, key)?;
        if !chain.starts_with(level0) {
            return None;
        }
        if let Ok(file) = fs::File::options().write(true).open(&path) {
            let _ = file.set_modified(std::time::SystemTime::now());
        }
        Some((chain.to_vec(), levels))
    }

    /// Stores the chain of `key`. Errors (a read-only or full disk) are logged and ignored: the
    /// cache only saves time.
    pub fn put(&self, key: &ChainKey, chain: &[u8], levels: u32) {
        if let Err(e) = self.write(key, chain, levels) {
            tracing::debug!(dir = %self.dir.display(), "mip chain cache write failed: {e}");
        }
    }

    fn write(&self, key: &ChainKey, chain: &[u8], levels: u32) -> std::io::Result<()> {
        static SERIAL: AtomicU64 = AtomicU64::new(0);
        fs::create_dir_all(&self.dir)?;
        let temp = self.dir.join(format!(
            ".{}.{}.{}.tmp",
            key.file_name(),
            std::process::id(),
            SERIAL.fetch_add(1, Relaxed)
        ));
        let mut file = fs::File::create(&temp)?;
        let mut header = Vec::with_capacity(HEADER);
        header.extend_from_slice(MAGIC);
        for v in [key.generator, key.format, key.width, key.height, levels] {
            header.extend_from_slice(&v.to_le_bytes());
        }
        header.extend_from_slice(&(chain.len() as u64).to_le_bytes());
        header.extend_from_slice(&checksum(chain).to_le_bytes());
        file.write_all(&header)?;
        file.write_all(chain)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp, self.dir.join(key.file_name())).inspect_err(|_| {
            let _ = fs::remove_file(&temp);
        })
    }

    /// Removes files of other generator versions and leftover temporary files, then the least
    /// recently used chains until the cache fits its cap. Returns the bytes left.
    pub fn prune(&self) -> u64 {
        let Ok(entries) = fs::read_dir(&self.dir) else {
            return 0;
        };
        let suffix = format!("-g{}.mip", self.generator);
        let mut files = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            let Ok(meta) = entry.metadata() else { continue };
            if !meta.is_file() {
                continue;
            }
            let stale_temp = name.ends_with(".tmp")
                && meta
                    .modified()
                    .ok()
                    .and_then(|m| m.elapsed().ok())
                    .is_some_and(|age| age.as_secs() > 3600);
            if stale_temp || (name.ends_with(".mip") && !name.ends_with(&suffix)) {
                let _ = fs::remove_file(&path);
            } else if name.ends_with(&suffix) {
                let used = meta.modified().unwrap_or(std::time::UNIX_EPOCH);
                files.push((used, meta.len(), path));
            }
        }
        let mut total: u64 = files.iter().map(|f| f.1).sum();
        files.sort_by_key(|f| f.0);
        for (_, len, path) in files {
            if total <= self.cap_bytes {
                break;
            }
            if fs::remove_file(&path).is_ok() {
                total -= len;
            }
        }
        total
    }
}

/// The chain and level count of a cache file, when its header and checksum agree with `key`.
fn parse<'a>(bytes: &'a [u8], key: &ChainKey) -> Option<(&'a [u8], u32)> {
    if bytes.len() < HEADER || &bytes[..8] != MAGIC {
        return None;
    }
    let word =
        |i: usize| u32::from_le_bytes(bytes[8 + 4 * i..12 + 4 * i].try_into().unwrap_or_default());
    let (generator, format, width, height, levels) = (word(0), word(1), word(2), word(3), word(4));
    let len = u64::from_le_bytes(bytes[28..36].try_into().unwrap_or_default());
    let sum = u64::from_le_bytes(bytes[36..44].try_into().unwrap_or_default());
    let payload = &bytes[HEADER..];
    let expected = chain_bytes(width, height, levels, key.block_bytes());
    if (generator, format, width, height) != (key.generator, key.format, key.width, key.height)
        || levels != full_chain_levels(width, height)
        || len != payload.len() as u64
        || payload.len() != expected
        || checksum(payload) != sum
    {
        return None;
    }
    Some((payload, levels))
}

/// The game's cache (`<cache dir>/opts/textures`, cap [`DEFAULT_CAP_BYTES`]); `None` when the
/// platform has no cache directory. The first call starts one prune on a background thread.
pub fn global() -> Option<&'static ChainCache> {
    static CACHE: OnceLock<Option<ChainCache>> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            let dir = directories::ProjectDirs::from("org", "opts", "opts")?
                .cache_dir()
                .join("textures");
            let cache = ChainCache::new(dir, DEFAULT_CAP_BYTES, super::bc_mips::GENERATOR_VERSION);
            let pruner = cache.clone();
            let _ = std::thread::Builder::new()
                .name("mip-cache-prune".into())
                .spawn(move || {
                    let left = pruner.prune();
                    tracing::debug!(bytes = left, dir = %pruner.dir.display(), "mip chain cache pruned");
                });
            Some(cache)
        })
        .as_ref()
}

#[cfg(test)]
#[path = "chain_cache/tests.rs"]
mod tests;
