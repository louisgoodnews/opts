//! Locating the user's OMSI 2 installation.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The installation found with an explicit `--omsi-path`, remembered for UI code that has no
/// handle on the session configuration.
static KNOWN_ROOT: OnceLock<PathBuf> = OnceLock::new();

/// Steam install locations below `$HOME`, tried after `--omsi-path` and `$OMSI_PATH`.
fn steam_candidates() -> Vec<PathBuf> {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return Vec::new();
    };
    [".steam/steam", ".local/share/Steam", ".steam/root"]
        .iter()
        .map(|steam| home.join(steam).join("steamapps/common/OMSI 2"))
        .collect()
}

/// Returns the OMSI installation root: `explicit` (CLI), then `$OMSI_PATH`, then the default
/// Steam locations.
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
        .chain(steam_candidates())
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
