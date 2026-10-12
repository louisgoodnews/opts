//! RON theme files in the `themes/` folder of the configuration directory.
//!
//! A file `<name>.ron` is chosen by the `theme_file` setting; it is read through the contrast
//! checker of `opts_gui::theme::file` (bad pairs are auto-corrected, hopeless files rejected).
//! Problems never stop the game: the built-in theme is used and the reason is logged.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use directories::ProjectDirs;
use opts_gui::theme::{file::parse_theme_file, CustomTheme};

/// The folder theme files are read from (`<config dir>/themes`).
pub fn themes_dir() -> Option<PathBuf> {
    ProjectDirs::from("org", "opts", "opts").map(|dirs| dirs.config_dir().join("themes"))
}

/// The names (file stems) of the `*.ron` files in `dir`, sorted.
pub fn discover_in(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            (p.extension().is_some_and(|x| x == "ron"))
                .then(|| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
                .flatten()
        })
        .collect();
    names.sort();
    names
}

/// The names of the available theme files.
pub fn discover_theme_files() -> Vec<String> {
    themes_dir().map(|d| discover_in(&d)).unwrap_or_default()
}

/// Loads and checks the theme file `name` from `dir`; `None` (with a log line) on any problem.
pub fn load_from(dir: &Path, name: &str) -> Option<Arc<CustomTheme>> {
    if name.is_empty() || name.contains(['/', '\\']) || name.starts_with('.') {
        return None;
    }
    let path = dir.join(format!("{name}.ron"));
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(err) => {
            tracing::warn!(?path, %err, "theme file cannot be read; using the built-in theme");
            return None;
        }
    };
    match parse_theme_file(&text) {
        Ok(theme) => {
            if theme.corrected.is_empty() {
                tracing::info!(?path, name = %theme.name, "theme file loaded");
            } else {
                tracing::warn!(
                    ?path,
                    corrected = ?theme.corrected,
                    "theme file loaded; some colours were adjusted to meet the contrast rules"
                );
            }
            Some(Arc::new(theme))
        }
        Err(err) => {
            tracing::warn!(?path, %err, "theme file rejected; using the built-in theme");
            None
        }
    }
}

/// Loads the theme file `name` from the configuration directory.
pub fn load_theme_file(name: &str) -> Option<Arc<CustomTheme>> {
    load_from(&themes_dir()?, name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir() -> PathBuf {
        let d = std::env::temp_dir().join(format!("opts-theme-files-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&d);
        d
    }

    #[test]
    fn files_are_discovered_loaded_and_bad_ones_ignored() {
        let d = dir();
        std::fs::write(d.join("good.ron"), r#"(version: 1, name: "Good")"#).expect("write");
        std::fs::write(d.join("bad.ron"), "nonsense").expect("write");
        std::fs::write(d.join("note.txt"), "x").expect("write");
        assert_eq!(discover_in(&d), ["bad", "good"]);
        assert!(load_from(&d, "good").is_some());
        assert!(load_from(&d, "bad").is_none());
        assert!(load_from(&d, "missing").is_none());
        assert!(load_from(&d, "../good").is_none(), "no path escapes");
        assert!(load_from(&d, "").is_none());
        let _ = std::fs::remove_dir_all(&d);
    }
}
