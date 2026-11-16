//! The cache directory: writing and reading the chains, pruning it, and the game's cache.

use super::parse::{checksum, parse};
use super::*;

/// A directory of cached chains with a size cap.
#[derive(Debug, Clone)]
pub struct ChainCache {
    pub(super) dir: PathBuf,
    pub(super) cap_bytes: u64,
    pub(super) generator: u32,
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

/// The game's cache (`<cache dir>/opts/textures`, cap [`DEFAULT_CAP_BYTES`]); `None` when the
/// platform has no cache directory. The first call starts one prune on a background thread.
pub fn global() -> Option<&'static ChainCache> {
    static CACHE: OnceLock<Option<ChainCache>> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            let dir = directories::ProjectDirs::from("org", "opts", "opts")?
                .cache_dir()
                .join("textures");
            let cache = ChainCache::new(dir, DEFAULT_CAP_BYTES, super::super::bc_mips::GENERATOR_VERSION);
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
