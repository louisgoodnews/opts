//! The Steam installations of this platform: the default folders and the Windows registry.

use std::path::PathBuf;

/// The Steam installations of this platform, most likely first (they may not exist).
#[must_use]
pub fn steam_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if cfg!(windows) {
        roots.extend(windows_registry_roots());
        for var in ["ProgramFiles(x86)", "ProgramFiles"] {
            if let Some(dir) = std::env::var_os(var) {
                roots.push(PathBuf::from(dir).join("Steam"));
            }
        }
    } else if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        let relative: &[&str] = if cfg!(target_os = "macos") {
            &["Library/Application Support/Steam"]
        } else {
            &[
                ".steam/steam",
                ".steam/root",
                ".local/share/Steam",
                ".var/app/com.valvesoftware.Steam/.local/share/Steam",
                ".var/app/com.valvesoftware.Steam/data/Steam",
                "snap/steam/common/.local/share/Steam",
            ]
        };
        roots.extend(relative.iter().map(|r| home.join(r)));
    }
    roots
}

/// Steam's folder from the Windows registry (`reg query`, part of every Windows).
pub(super) fn windows_registry_roots() -> Vec<PathBuf> {
    [
        (r"HKCU\Software\Valve\Steam", "SteamPath"),
        (r"HKLM\SOFTWARE\WOW6432Node\Valve\Steam", "InstallPath"),
        (r"HKLM\SOFTWARE\Valve\Steam", "InstallPath"),
    ]
    .iter()
    .filter_map(|(key, value)| {
        let out = std::process::Command::new("reg")
            .args(["query", key, "/v", value])
            .output()
            .ok()?;
        out.status.success().then_some(())?;
        registry_value(&String::from_utf8_lossy(&out.stdout), value).map(PathBuf::from)
    })
    .collect()
}

/// The data of `value` in the output of `reg query <key> /v <value>`
/// (`    SteamPath    REG_SZ    c:/program files (x86)/steam`).
pub(super) fn registry_value(output: &str, value: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let rest = line.trim_start().strip_prefix(value)?;
        let rest = rest.trim_start();
        let (kind, data) = rest.split_once(char::is_whitespace)?;
        kind.starts_with("REG_")
            .then(|| data.trim().to_owned())
            .filter(|d| !d.is_empty())
    })
}
