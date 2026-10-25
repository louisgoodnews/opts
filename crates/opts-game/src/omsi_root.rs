//! Locating the user's OMSI 2 installation.
//!
//! Search order (`docs/src/architecture/application-startup.md`): `--omsi-path` (or the root of
//! `--map-path`), `$OMSI_PATH`, the path remembered in the settings. Steam's library configuration
//! ([`crate::steam_library`]) is read once, by the startup check ([`crate::omsi_setup`]), which
//! remembers what it finds; when nothing is found the startup dialog asks the user.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// The installation found with an explicit `--omsi-path`, remembered for UI code that has no
/// handle on the session configuration.
static KNOWN_ROOT: OnceLock<PathBuf> = OnceLock::new();

/// The path stored in the settings (set at startup, replaced when the user picks another).
static SAVED_ROOT: Mutex<Option<PathBuf>> = Mutex::new(None);

/// The installation Steam knows, searched once per process.
static STEAM_ROOT: OnceLock<Option<PathBuf>> = OnceLock::new();

/// The OMSI 2 installation named by Steam's library configuration, if any (for the startup check;
/// everything else reads the remembered path).
#[must_use]
pub fn steam_root() -> Option<PathBuf> {
    STEAM_ROOT
        .get_or_init(|| crate::steam_library::find_omsi(&crate::steam_library::steam_roots()))
        .clone()
}

/// Whether `dir` looks like an OMSI 2 installation: it has the `maps` and `Vehicles` folders.
#[must_use]
pub fn is_omsi_root(dir: &Path) -> bool {
    dir.join("maps").is_dir() && dir.join("Vehicles").is_dir()
}

/// Resolves a folder the user picked: the folder itself or, when they picked a sub-folder such as
/// `maps/Grundorf` or `Vehicles`, the installation above it.
#[must_use]
pub fn root_from_selection(picked: &Path) -> Option<PathBuf> {
    picked
        .ancestors()
        .take(4)
        .find(|p| is_omsi_root(p))
        .map(Path::to_path_buf)
}

/// Remembers the path stored in the settings (an invalid path is ignored).
pub fn set_saved_root(path: Option<PathBuf>) {
    if let Ok(mut saved) = SAVED_ROOT.lock() {
        *saved = path.filter(|p| is_omsi_root(p));
    }
}

fn saved_root() -> Option<PathBuf> {
    SAVED_ROOT.lock().ok().and_then(|s| s.clone())
}

/// Returns the OMSI installation root: `explicit` (CLI), then `$OMSI_PATH`, then the path saved
/// in the settings (which holds the installation Steam named once the startup check adopted it).
///
/// A directory only counts as an installation if it contains a `maps` folder. The first
/// installation found through `explicit` is remembered for [`known_omsi_root`].
#[must_use]
pub fn locate_omsi_root(explicit: Option<&Path>) -> Option<PathBuf> {
    let env = std::env::var_os("OMSI_PATH").map(PathBuf::from);
    let found = explicit
        .map(Path::to_path_buf)
        .into_iter()
        .chain(env)
        .chain(saved_root())
        .find(|p| p.join("maps").is_dir());
    if let (Some(root), Some(_)) = (&found, explicit) {
        let _ = KNOWN_ROOT.set(root.clone());
    }
    found
}

/// The installation of this process: the one given on the command line, else the default search.
#[must_use]
pub fn known_omsi_root() -> Option<PathBuf> {
    KNOWN_ROOT.get().cloned().or_else(|| locate_omsi_root(None))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_install(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("opts-root-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(root.join("maps/Grundorf")).expect("mkdir");
        std::fs::create_dir_all(root.join("Vehicles")).expect("mkdir");
        root
    }

    #[test]
    fn an_install_needs_maps_and_vehicles() {
        let root = fake_install("valid");
        assert!(is_omsi_root(&root));
        assert!(!is_omsi_root(&root.join("maps")));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_picked_subfolder_resolves_to_the_install() {
        let root = fake_install("sub");
        assert_eq!(root_from_selection(&root), Some(root.clone()));
        assert_eq!(
            root_from_selection(&root.join("maps/Grundorf")),
            Some(root.clone())
        );
        assert_eq!(
            root_from_selection(&root.join("Vehicles")),
            Some(root.clone())
        );
        assert_eq!(root_from_selection(&std::env::temp_dir()), None);
        let _ = std::fs::remove_dir_all(root);
    }
}
