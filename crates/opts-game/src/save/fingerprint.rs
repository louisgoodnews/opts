//! Content fingerprints: a short hash of the map and of each vehicle a save refers to, so a load
//! can say that the content changed since the save (a warning, never a refusal).
//!
//! FNV-1a 64 bit: not cryptographic, stable across platforms and builds, no dependency.

use std::fs;
use std::path::Path;

/// FNV-1a over `bytes`, as 16 hex digits.
#[must_use]
pub fn hash_bytes(bytes: &[u8]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}")
}

/// The fingerprint of a map folder: its `global.cfg` and the names and sizes of the tile files
/// (`*.map`) beside it. `None` when the folder or `global.cfg` is missing.
#[must_use]
pub fn map(dir: &Path) -> Option<String> {
    let mut data = fs::read(dir.join("global.cfg")).ok()?;
    let mut tiles: Vec<(String, u64)> = fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            name.to_ascii_lowercase()
                .ends_with(".map")
                .then(|| (name, e.metadata().map_or(0, |m| m.len())))
        })
        .collect();
    tiles.sort();
    for (name, size) in tiles {
        data.extend_from_slice(format!("\n{name}:{size}").as_bytes());
    }
    Some(hash_bytes(&data))
}

/// The fingerprint of a vehicle: its `.bus` file. `None` when it cannot be read.
#[must_use]
pub fn vehicle(bus_file: &Path) -> Option<String> {
    fs::read(bus_file).ok().map(|b| hash_bytes(&b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hash_is_stable_and_sensitive() {
        assert_eq!(hash_bytes(b""), "cbf29ce484222325");
        assert_eq!(hash_bytes(b"a"), "af63dc4c8601ec8c");
        assert_ne!(hash_bytes(b"ab"), hash_bytes(b"ba"));
    }

    #[test]
    fn a_map_changes_when_a_tile_is_added() {
        let dir = std::env::var_os("HOME")
            .map_or_else(std::env::temp_dir, std::path::PathBuf::from)
            .join(".local/share/opts/benchmark/session-save/fingerprint-test");
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(map(&dir), None);
        fs::create_dir_all(&dir).expect("dir");
        assert_eq!(map(&dir), None, "no global.cfg");
        fs::write(dir.join("global.cfg"), "x").expect("cfg");
        let a = map(&dir).expect("hash");
        fs::write(dir.join("tile_0_0.map"), "123").expect("tile");
        let b = map(&dir).expect("hash");
        assert_ne!(a, b);
        assert_eq!(map(&dir), Some(b));
        let _ = fs::remove_dir_all(&dir);
    }
}
