//! Keeps tests out of the user's configuration.
//!
//! Settings, the drive history and the first-run flag live below the XDG directories. A test that
//! starts a drive or saves a setting would otherwise write to the real `~/.config/opts`. Tests call
//! [`isolate_user_dirs`] before they build an app; it points the XDG variables at a throwaway
//! directory below the benchmark folder (the same one every time, so nothing accumulates).

use std::sync::Once;

/// Points `XDG_CONFIG_HOME`, `XDG_DATA_HOME` and `XDG_CACHE_HOME` at a throwaway directory. Safe to
/// call from every test; only the first call does anything.
pub fn isolate_user_dirs() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let base = std::env::var_os("HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join(".local/share/opts/benchmark/dev-ui-impl/shell/test-xdg");
        for (var, dir) in [
            ("XDG_CONFIG_HOME", "config"),
            ("XDG_DATA_HOME", "data"),
            ("XDG_CACHE_HOME", "cache"),
        ] {
            let path = base.join(dir);
            let _ = std::fs::create_dir_all(&path);
            std::env::set_var(var, path);
        }
    });
}
