//! The optional local profile file. The caller chooses the folder; nothing here knows a path of
//! the user's system. The file is plain JSON the user can read and delete.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::profile::{HardwareProfile, LoadError};

/// File name inside the folder.
pub const FILE_NAME: &str = "hardware.json";

/// Full path of the profile file in `dir`.
#[must_use]
pub fn path(dir: &Path) -> PathBuf {
    dir.join(FILE_NAME)
}

/// Writes the profile (via a temporary file, so a crash never leaves half a file).
///
/// # Errors
/// I/O errors.
pub fn save(dir: &Path, profile: &HardwareProfile) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let tmp = dir.join(format!("{FILE_NAME}.tmp"));
    fs::write(&tmp, profile.to_json())?;
    fs::rename(&tmp, path(dir))
}

/// Reads the stored profile; `None` when absent or unreadable (an unreadable file is ignored,
/// the probe simply runs again).
#[must_use]
pub fn load(dir: &Path) -> Option<HardwareProfile> {
    let text = fs::read_to_string(path(dir)).ok()?;
    match HardwareProfile::from_json(&text) {
        Ok(p) => Some(p),
        Err(LoadError::Parse(_) | LoadError::TooNew(_)) => None,
    }
}

/// Deletes the stored profile; `true` when a file was removed.
#[must_use]
pub fn delete(dir: &Path) -> bool {
    fs::remove_file(path(dir)).is_ok()
}

/// Keeps the benchmark results of `cached` when they were measured on the same hardware class as
/// `fresh`.
#[must_use]
pub fn with_cached_bench(mut fresh: HardwareProfile, cached: Option<&HardwareProfile>) -> HardwareProfile {
    if let Some(c) = cached {
        let class = fresh.class_hash();
        if c.bench.measured_for_class.known().is_some_and(|h| *h == class) {
            fresh.bench = c.bench.clone();
        }
    }
    fresh
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fact::Fact;

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("opts-hw-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn save_load_delete() {
        let d = scratch("store");
        let mut p = HardwareProfile::default();
        p.cpu.logical_cores = Fact::Known(8);
        save(&d, &p).unwrap();
        assert_eq!(load(&d), Some(p));
        assert!(delete(&d));
        assert!(!delete(&d));
        assert_eq!(load(&d), None);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn garbage_and_too_new_files_are_ignored() {
        let d = scratch("garbage");
        fs::create_dir_all(&d).unwrap();
        fs::write(path(&d), "not json").unwrap();
        assert_eq!(load(&d), None);
        fs::write(path(&d), r#"{"schema":99}"#).unwrap();
        assert_eq!(load(&d), None);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn bench_is_reused_only_for_the_same_class() {
        let mut cached = HardwareProfile::default();
        cached.bench.cpu_single_mops = Fact::Known(900);
        cached.bench.measured_for_class = Fact::Known(cached.class_hash());
        let same = with_cached_bench(HardwareProfile::default(), Some(&cached));
        assert_eq!(same.bench.cpu_single_mops, Fact::Known(900));
        let mut other = HardwareProfile::default();
        other.memory.total_mib = Fact::Known(65536);
        assert_eq!(with_cached_bench(other, Some(&cached)).bench.cpu_single_mops, Fact::default());
    }
}
