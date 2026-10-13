//! The only paths the probe may read. A path outside this list is refused before any I/O
//! (`Reason::Unavailable`), so the audit of "what does the crate read" is this one table.
//! Compiled on every platform so the table is tested everywhere.
#![cfg_attr(
    not(any(target_os = "linux", test)),
    allow(
        dead_code,
        reason = "only the Linux probe reads files; the table is tested everywhere"
    )
)]

use std::fs;
use std::io::Read;
use std::path::Path;

use crate::fact::Reason;

/// Readable files. `*` matches one path segment (no `/`). Only the listed facts are taken from
/// them; the rest of the text is dropped after parsing.
pub const READ: &[(&str, &str)] = &[
    ("/proc/cpuinfo", "the `hypervisor` flag only"),
    ("/proc/self/cgroup", "container keywords only"),
    ("/proc/asound/cards", "number of sound cards only"),
    ("/proc/bus/input/devices", "number of `js*` handlers only"),
    (
        "/sys/devices/system/cpu/cpu0/cache/index*/level",
        "cache level",
    ),
    (
        "/sys/devices/system/cpu/cpu0/cache/index*/type",
        "cache type",
    ),
    (
        "/sys/devices/system/cpu/cpu0/cache/index*/size",
        "cache size",
    ),
    (
        "/sys/class/drm/card*/device/vendor",
        "PCI vendor id, classified",
    ),
    (
        "/sys/class/drm/card*/device/mem_info_vram_total",
        "video memory (amdgpu)",
    ),
    ("/sys/class/drm/card*-*/status", "connected or not"),
    ("/sys/class/drm/card*-*/modes", "preferred resolution only"),
    ("/sys/class/dmi/id/sys_vendor", "hypervisor keyword only"),
    ("/sys/class/dmi/id/product_name", "hypervisor keyword only"),
];

/// Directories whose entry *names* may be listed.
pub const LIST: &[&str] = &["/sys/class/drm", "/sys/devices/system/cpu/cpu0/cache"];

/// Paths whose existence (not content) may be tested.
pub const EXISTS: &[&str] = &["/.dockerenv", "/run/.containerenv"];

/// Largest file the probe reads (the files above are tiny; `/proc/cpuinfo` can reach a few MiB
/// on many-core machines, of which the first 256 KiB hold the flags).
const MAX_BYTES: u64 = 256 * 1024;

/// `*` matches within one segment.
fn glob(pattern: &str, path: &str) -> bool {
    let (p, s): (Vec<&str>, Vec<&str>) = (pattern.split('/').collect(), path.split('/').collect());
    p.len() == s.len() && p.iter().zip(&s).all(|(p, s)| seg(p, s))
}

fn seg(p: &str, s: &str) -> bool {
    // `*` matches any run of characters of the segment (segments never contain `/`).
    match p.split_once('*') {
        None => p == s,
        Some((head, rest)) => s.strip_prefix(head).is_some_and(|tail| {
            (0..=tail.len())
                .filter(|i| tail.is_char_boundary(*i))
                .any(|i| seg(rest, &tail[i..]))
        }),
    }
}

/// May `path` be read?
#[must_use]
pub fn may_read(path: &str) -> bool {
    !path.contains("..") && READ.iter().any(|(p, _)| glob(p, path))
}

/// Reads an allowlisted file (at most 256 KiB).
///
/// # Errors
/// `Unavailable` when not allowlisted or missing, `Unreadable` on other I/O errors.
pub fn read(path: &str) -> Result<String, Reason> {
    if !may_read(path) {
        return Err(Reason::Unavailable);
    }
    let mut buf = Vec::new();
    fs::File::open(path)
        .and_then(|f| f.take(MAX_BYTES).read_to_end(&mut buf))
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => Reason::Unavailable,
            _ => Reason::Unreadable,
        })?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

/// Entry names of an allowlisted directory.
///
/// # Errors
/// As [`read`].
pub fn list(dir: &str) -> Result<Vec<String>, Reason> {
    if !LIST.contains(&dir) {
        return Err(Reason::Unavailable);
    }
    let rd = fs::read_dir(dir).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => Reason::Unavailable,
        _ => Reason::Unreadable,
    })?;
    let mut v: Vec<String> = rd
        .filter_map(Result::ok)
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    v.sort();
    Ok(v)
}

/// Does an allowlisted path exist?
#[must_use]
pub fn exists(path: &str) -> bool {
    EXISTS.contains(&path) && Path::new(path).exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlist_accepts_listed_and_refuses_the_rest() {
        assert!(may_read("/proc/cpuinfo"));
        assert!(may_read("/sys/class/drm/card0/device/vendor"));
        assert!(may_read("/sys/class/drm/card1-HDMI-A-1/modes"));
        assert!(may_read("/sys/devices/system/cpu/cpu0/cache/index3/size"));
        for bad in [
            "/etc/passwd",
            "/etc/hostname",
            "/proc/self/environ",
            "/sys/class/net/eth0/address",
            "/sys/class/dmi/id/product_serial",
            "/sys/class/dmi/id/product_uuid",
            "/sys/class/drm/card0/device/../../../../etc/passwd",
            "/sys/class/drm/card0/x/device/vendor",
            "/proc/cpuinfo/x",
        ] {
            assert!(!may_read(bad), "{bad} must be refused");
        }
        assert_eq!(read("/etc/hostname"), Err(Reason::Unavailable));
        assert_eq!(list("/home"), Err(Reason::Unavailable));
        assert!(!exists("/etc/passwd"));
    }

    #[test]
    fn no_allowlisted_path_names_an_identifier() {
        for (p, _) in READ {
            for word in ["serial", "uuid", "address", "hostname", "environ", "mac"] {
                assert!(!p.contains(word), "{p}");
            }
        }
    }
}
