//! The libraries of a Steam installation and the OMSI 2 folder in them.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::kv::*;
use super::{DEFAULT_INSTALL_DIR, OMSI2_APP_ID};
use crate::omsi_root::is_omsi_root;

/// One Steam library: its folder and whether its `apps` list names OMSI 2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Library {
    /// The library folder (it holds `steamapps/`).
    pub path: PathBuf,
    /// Whether Steam lists OMSI 2 in it (only known in the current file format).
    pub has_omsi: bool,
}

/// The libraries of a `libraryfolders.vdf`, in both the current format
/// (`"0" { "path" "…" "apps" { … } }`) and the one before 2021 (`"1" "D:\\SteamLibrary"`).
#[must_use]
pub fn libraries(vdf: &str) -> Vec<Library> {
    let root = parse(vdf);
    let Some(Kv::Block(entries)) = root.get("libraryfolders") else {
        return Vec::new();
    };
    entries
        .iter()
        .filter(|(key, _)| key.chars().all(|c| c.is_ascii_digit()))
        .filter_map(|(_, value)| match value {
            Kv::Str(path) => Some(Library {
                path: PathBuf::from(path),
                has_omsi: false,
            }),
            Kv::Block(_) => Some(Library {
                path: PathBuf::from(value.get("path")?.as_str()?),
                has_omsi: value
                    .get("apps")
                    .is_some_and(|apps| apps.get(OMSI2_APP_ID).is_some()),
            }),
        })
        .collect()
}

/// The `installdir` of an `appmanifest_<appid>.acf`.
#[must_use]
pub fn install_dir(acf: &str) -> Option<String> {
    parse(acf)
        .get("AppState")?
        .get("installdir")?
        .as_str()
        .map(str::to_owned)
        .filter(|d| !d.trim().is_empty())
}

/// The OMSI 2 installation known to the Steam installations `steam_roots`, if any: the
/// libraries that list OMSI 2 first, then the others (each installation's own folder included).
#[must_use]
pub fn find_omsi(steam_roots: &[PathBuf]) -> Option<PathBuf> {
    let mut listed = Vec::new();
    let mut others = Vec::new();
    let mut seen = HashSet::new();
    let mut add = |path: PathBuf, has_omsi: bool| {
        let key = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        if seen.insert(key) {
            if has_omsi {
                listed.push(path);
            } else {
                others.push(path);
            }
        }
    };
    for steam in steam_roots.iter().filter(|p| p.is_dir()) {
        let found: Vec<Library> = ["steamapps/libraryfolders.vdf", "config/libraryfolders.vdf"]
            .iter()
            .filter_map(|f| std::fs::read_to_string(steam.join(f)).ok())
            .flat_map(|text| libraries(&text))
            .collect();
        add(
            steam.clone(),
            found.iter().any(|l| l.has_omsi && same(&l.path, steam)),
        );
        for lib in found {
            add(lib.path, lib.has_omsi);
        }
    }
    listed
        .into_iter()
        .chain(others)
        .find_map(|library| omsi_in_library(&library))
}

/// Whether two paths name the same folder (symbolic links resolved where possible).
pub(super) fn same(a: &Path, b: &Path) -> bool {
    let canon = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    canon(a) == canon(b)
}

/// OMSI 2 in one library: the folder its manifest names, else the default one.
pub(super) fn omsi_in_library(library: &Path) -> Option<PathBuf> {
    let steamapps = library.join("steamapps");
    let dir = std::fs::read_to_string(steamapps.join(format!("appmanifest_{OMSI2_APP_ID}.acf")))
        .ok()
        .and_then(|acf| install_dir(&acf))
        .unwrap_or_else(|| DEFAULT_INSTALL_DIR.to_owned());
    let root = steamapps.join("common").join(dir);
    is_omsi_root(&root).then_some(root)
}
