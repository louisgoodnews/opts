//! The cache of a map's static load profile: the file of counts and the signature that proves it is still current.

use super::*;

#[derive(Debug, Serialize, Deserialize)]
struct CacheFile {
    tiles: usize,
    bytes: u64,
    objects: usize,
    splines: usize,
    ai_pool: usize,
}

/// A fingerprint of the tile files: how many and how big.
pub(super) fn signature(map_dir: &Path) -> (usize, u64) {
    let files = tile_files(map_dir);
    (files.len(), files.iter().map(|f| f.1).sum())
}

pub(super) fn cache_path(store: &ProfileStore, map_id: &str) -> PathBuf {
    store.root().join(map_id).join("static.toml")
}

/// The cached counts when the tile files are as they were when they were counted.
pub(super) fn read_cache(store: &ProfileStore, map_id: &str, sig: (usize, u64)) -> Option<StaticProfile> {
    let c: CacheFile =
        toml::from_str(&std::fs::read_to_string(cache_path(store, map_id)).ok()?).ok()?;
    ((c.tiles, c.bytes) == sig).then_some(StaticProfile {
        tiles: c.tiles,
        objects: c.objects,
        splines: c.splines,
        ai_pool: c.ai_pool,
        estimated: false,
    })
}

pub(super) fn write_cache(store: &ProfileStore, map_id: &str, sig: (usize, u64), p: &StaticProfile) {
    let c = CacheFile {
        tiles: sig.0,
        bytes: sig.1,
        objects: p.objects,
        splines: p.splines,
        ai_pool: p.ai_pool,
    };
    let path = cache_path(store, map_id);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(text) = toml::to_string_pretty(&c) {
        let _ = std::fs::write(path, text);
    }
}
